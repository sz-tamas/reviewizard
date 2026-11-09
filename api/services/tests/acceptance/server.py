"""Disposable application process and HTTP client for native acceptance checks."""
import http.client
import json
import os
import queue
import re
import signal
import socket
import subprocess
import threading
import urllib.error
import urllib.request

def normalize(value):
    if isinstance(value, dict):
        return {k: '<timestamp>' if k in ('createdAt', 'updatedAt') else normalize(v) for k, v in value.items()}
    if isinstance(value, list):
        return [normalize(v) for v in value]
    return value


class Backend:
    def __init__(self, binary, workspace, stage, env, data=None):
        self.data = data if data is not None else stage / 'data'
        with socket.socket() as reservation:
            reservation.bind(('127.0.0.1', 0))
            port = str(reservation.getsockname()[1])
        flags = ['--root', str(workspace), '--port', port, '--data-dir', str(self.data), '--retention-days', '0']
        self.proc = subprocess.Popen([str(binary), *flags], cwd=stage, env=env, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True)
        self.messages = queue.Queue()
        self.logs = []
        def read_log():
            for line in self.proc.stderr:
                self.logs.append(line.rstrip())
                self.messages.put(line)
            self.messages.put(None)
        threading.Thread(target=read_log, daemon=True).start()
        try:
            while True:
                line = self.messages.get(timeout=20)
                if line is None:
                    raise RuntimeError(f'Application exited before binding: {self.logs}')
                match = re.search(r'listening on (http://127\.0\.0\.1:\d+)', line)
                if match:
                    self.url = match[1]
                    break
        except BaseException:
            self.close()
            raise

    def call(self, method, path, payload=None, headers=None):
        data = None if payload is None else json.dumps(payload).encode()
        h = {'Host': self.url.removeprefix('http://')}
        if data is not None:
            h['Content-Type'] = 'application/json'
        h.update(headers or {})
        request = urllib.request.Request(self.url + path, data=data, headers=h, method=method)
        try:
            response = urllib.request.urlopen(request, timeout=15)
        except urllib.error.HTTPError as error:
            response = error
        with response:
            body = response.read()
            result = {'status': response.status, 'body': body, 'headers': dict(response.headers.items())}
            if 'application/json' in response.headers.get('Content-Type', '') and body:
                result['json'] = json.loads(body)
            return result

    def close(self):
        if self.proc.poll() is None:
            self.proc.send_signal(signal.SIGTERM)
            try:
                self.proc.wait(timeout=8)
            except subprocess.TimeoutExpired:
                self.proc.kill()
                self.proc.wait()
        self.proc.stderr.close()

