#!/usr/bin/env python3
"""Standalone application, persistence, snapshot, SSE and shutdown acceptance (POSIX)."""
import argparse
import hashlib
import http.client
import json
import os
from pathlib import Path
from urllib.parse import urlencode
import shutil
import socket
import sqlite3
import subprocess
import tempfile
import time
from server import Backend, normalize


def rows(path):
    connection=sqlite3.connect(path.as_uri()+'?mode=ro',uri=True)
    try:
        assert connection.execute('PRAGMA integrity_check').fetchone()==('ok',)
        assert not connection.execute('PRAGMA foreign_key_check').fetchall()
        return {table:connection.execute('SELECT * FROM '+table+' ORDER BY 1,2').fetchall()
                for table in ['reviews','comments','replies','reviewed_files']}
    finally:connection.close()


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary',type=Path,required=True)
    parser.add_argument('--report',type=Path,default=Path('target/acceptance/results.json'))
    args=parser.parse_args()
    if os.name=='nt':raise SystemExit('POSIX process/signal acceptance; run native Windows smoke separately')
    source_binary=args.binary.resolve()
    report={'checks':[],'binary_sha256':hashlib.sha256(source_binary.read_bytes()).hexdigest()}
    backend=None
    def check(name,condition):
        report['checks'].append({'name':name,'passed':bool(condition)})
        assert condition,name
    try:
        with tempfile.TemporaryDirectory(prefix='reviewizard-default-') as directory:
            stage=Path(directory).resolve();workspace=stage/'workspace';repo=workspace/'demo';repo.mkdir(parents=True)
            env=dict(os.environ,GIT_CONFIG_GLOBAL='/dev/null',GIT_CONFIG_SYSTEM='/dev/null')
            def git(*args):subprocess.run(['git','-C',str(repo),*args],env=env,check=True,stdout=subprocess.DEVNULL,stderr=subprocess.PIPE)
            git('init','-b','main');git('config','user.name','Acceptance');git('config','user.email','acceptance@example.invalid')
            (repo/'demo.txt').write_text('before\n');git('add','.');git('commit','-m','base');git('switch','-c','feature')
            (repo/'demo.txt').write_text('after\n');git('add','.');git('commit','-m','feature')
            backend=Backend(source_binary,workspace,stage,env)
            def call(method,path,body=None,status=200):
                result=backend.call(method,path,body);assert result['status']==status,(path,result)
                return result.get('json')
            review=call('POST','/api/reviews',{'repo':'demo','head':'feature','base':'main'});rid=review['id']
            comment=call('POST',f'/api/reviews/{rid}/comments',{'filePath':'demo.txt','startLine':1,'endLine':1,'body':'Original anchor','side':'head','type':'bug'})
            cid=comment['id'];call('POST',f'/api/comments/{cid}/replies',{'body':'Original reply'})
            call('POST',f'/api/reviews/{rid}/reviewed',{'filePaths':['demo.txt'],'reviewed':True,'side':'head'},204)
            call('POST',f'/api/reviews/{rid}/summary',{'summary':'Original summary'},204)
            original=normalize(call('GET',f'/api/reviews/{rid}'));backend.close();backend=None
            # Model an imported database whose initialization receipt is absent.
            (stage/'data/rust-transition.json').unlink()
            database=stage/'data/reviewizard.db';original_rows=rows(database)
            # Copy the release binary to an empty runtime directory. No source/frontend files.
            standalone=stage/'standalone';standalone.mkdir();binary=standalone/'reviewizard';shutil.copy2(source_binary,binary)
            check('canonical version output',subprocess.check_output([str(binary),'--version'],cwd=standalone,text=True).startswith('reviewizard '))
            # Resume the same data using the copied standalone executable.
            backend=Backend(binary,workspace,stage,env)
            check('Rust identifies itself',call('GET','/healthz')['backend']=='rust')
            check('standalone resumes all review state',normalize(call('GET',f'/api/reviews/{rid}'))==original)
            backup=stage/'data/reviewizard.before-rust.db';backup_hash=hashlib.sha256(backup.read_bytes()).hexdigest()
            check('consistent backup preserves all four tables',rows(backup)==original_rows)
            check('read/migration preserves original rows',rows(database)==original_rows)
            root=backend.call('GET','/');check('backend does not serve UI',root['status']==404 and root['json']['error']=='not found')
            # Stream remains live for longer than normal HTTP writes; summary emits meta.
            stream=http.client.HTTPConnection('127.0.0.1',int(backend.url.rsplit(':',1)[1]),timeout=15)
            stream.request('GET',f'/api/reviews/{rid}/events');response=stream.getresponse()
            check('SSE connects',response.status==200 and b'connected' in response.readline());response.readline()
            call('POST',f'/api/reviews/{rid}/summary',{'summary':'Rust summary'},204)
            check('SSE mutation notification',response.readline().strip()==b'data: meta')
            rust_review=normalize(call('GET',f'/api/reviews/{rid}'))
            port=int(backend.url.rsplit(':',1)[1]);partial=socket.create_connection(('127.0.0.1',port),timeout=15)
            started=time.monotonic();partial.sendall(b'GET / HTTP/1.1\r\nHost: localhost\r\n')
            check('10-second header timeout',partial.recv(1024)==b'' and 8<=time.monotonic()-started<=14);partial.close()
            started=time.monotonic();backend.close();backend=None
            check('shutdown with open SSE is bounded',time.monotonic()-started<6)
            response.close();stream.close()
            backend=Backend(source_binary,workspace,stage,env)
            check('restart reads persisted changes',normalize(call('GET',f'/api/reviews/{rid}'))==rust_review)
            call('POST',f'/api/comments/{cid}/replies',{'body':'Reply after restart'})
            rollback_review=normalize(call('GET',f'/api/reviews/{rid}'));backend.close();backend=None
            backend=Backend(binary,workspace,stage,env)
            check('restart preserves the latest reply',normalize(call('GET',f'/api/reviews/{rid}'))==rollback_review)
            check('first transition backup is never overwritten',hashlib.sha256(backup.read_bytes()).hexdigest()==backup_hash)
            backend.close();backend=None
            restored=stage/'restored-data';restored.mkdir();shutil.copy2(backup,restored/'reviewizard.db')
            backend=Backend(source_binary,workspace,stage,env,data=restored)
            check('restore resumes exact snapshot review',normalize(call('GET',f'/api/reviews/{rid}'))==original)
            # Explicit projects work outside the startup root, without moving existing review identities.
            project=call('POST','/api/projects',{'path':str(repo)})
            check('registered folder identity',project['path']==str(repo) and project['reviewCount']==1)
            selected=f"@project/{project['id']}"
            same=call('POST','/api/reviews',{'repo':selected,'head':'feature','base':'main'})
            check('registering an existing folder preserves review IDs',same['id']==rid and same['repoPath']==str(repo))
            outside=stage/'outside'/'demo';shutil.copytree(repo,outside)
            second=call('POST','/api/projects',{'path':str(outside)})
            check('same named projects have distinct IDs',second['name']==project['name'] and second['id']!=project['id'])
            second_key=f"@project/{second['id']}"
            selected_review=call('POST','/api/reviews',{'repo':second_key,'head':'feature','base':'main'})
            check('second review has its own repository identity',selected_review['repoPath']==str(outside) and selected_review['id']!=rid)
            read=call('GET','/api/file?'+urlencode({'repo':second_key,'path':'demo.txt','ref':'feature'}))
            check('registered repository reads are scoped',read['content']=='after\n')
            call('POST','/api/projects',{'path':str(outside/'missing')},404)
            call('POST','/api/projects',{'path':'../demo'},400)
            call('DELETE',f"/api/projects/{second['id']}",status=204)
            check('removing project keeps its review',call('GET',f"/api/reviews/{selected_review['id']}")['repoPath']==str(outside))
            backend.close();backend=None
            backend=Backend(source_binary,workspace,stage,env,data=restored)
            check('project list survives restart',[(p['id'],p['path']) for p in call('GET','/api/projects')]==[(project['id'],str(repo))])
            check('registered identity resolves after restart',any(b['name']=='feature' for b in call('GET','/api/branches?'+urlencode({'repo':selected}))['branches']))
            readded=call('POST','/api/projects',{'path':str(outside)})
            check('re-adding preserves project and review IDs',readded['id']==second['id'] and readded['reviewCount']==1)
            # Saved comparisons persist their inclusive starting commit and source side.
            (repo/'demo.txt').write_text('index-only\n');git('add','demo.txt')
            (repo/'demo.txt').write_text('worktree-only\n')
            scoped=[]
            for side,expected in [('head','after'),('index','index-only'),('worktree','worktree-only')]:
                assignment=call('POST',f"/api/projects/{project['id']}/reviews",{'head':'feature','base':'main','from':'feature','side':side})
                check(f'{side} assignment records its scope',assignment['scopeSide']==side and assignment['scopeFrom']!='all' and assignment['scopeConfigured'])
                diff=call('GET',f"/api/reviews/{assignment['id']}/diff")
                check(f'{side} saved diff reads its selected source',expected in json.dumps(diff))
                scoped.append(assignment)
            check('scopes on the same branch have separate review IDs',len({r['id'] for r in scoped})==3)
            for side,expected in [('head','after'),('index','index-only'),('worktree','worktree-only')]:
                assignment=call('POST',f"/api/projects/{project['id']}/reviews",{'mode':'codebase','head':'feature','side':side})
                diff=call('GET',f"/api/reviews/{assignment['id']}/diff")
                check(f'{side} codebase scope inventories its complete source',assignment['scopeMode']=='codebase' and assignment['baseRef']=='' and expected in json.dumps(diff))
                scoped.append(assignment)
            backend.close();backend=None
            backend=Backend(source_binary,workspace,stage,env,data=restored)
            for saved in scoped:
                reopened=call('GET',f"/api/reviews/{saved['id']}")
                check(f"{saved['scopeSide']} scope survives process restart",all(reopened[k]==saved[k] for k in ['id','repoPath','baseRef','headRef','scopeFrom','scopeSide','scopeMode','projectId']))
            listed=call('GET',f"/api/projects/{project['id']}/reviews")
            check('Projects lists all saved review assignments',all(any(row['id']==saved['id'] for row in listed) for saved in scoped))
    except BaseException as error:
        report['incomplete']=repr(error)
        raise
    finally:
        if backend:backend.close()
        args.report.parent.mkdir(parents=True,exist_ok=True);args.report.write_text(json.dumps(report,indent=2)+'\n')
    print('Passed',len(report['checks']),'standalone/persistence checks. Report:',args.report)


if __name__=='__main__':main()
