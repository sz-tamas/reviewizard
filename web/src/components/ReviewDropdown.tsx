import { Check, ChevronDown } from "lucide-react";
import { DropdownMenu } from "radix-ui";

interface Props {
  label: string;
  value: string;
  options: { value: string; label: string }[];
  onChange: (value: string) => void;
  disabled?: boolean;
  placeholder?: string;
  fullWidth?: boolean;
}

export function ReviewDropdown({ label, value, options, onChange, disabled, placeholder, fullWidth }: Props) {
  const selected = options.find((option) => option.value === value)?.label ?? placeholder ?? value;
  return (
    <DropdownMenu.Root>
      <DropdownMenu.Trigger asChild>
        <button
          type="button"
          className={`review-action review-source-trigger${fullWidth ? " review-setup-select" : ""}`}
          aria-label={label}
          title={fullWidth ? selected : undefined}
          disabled={disabled}
        >
          <span>{selected}</span><ChevronDown size={17} aria-hidden="true" />
        </button>
      </DropdownMenu.Trigger>
      <DropdownMenu.Portal>
        <DropdownMenu.Content className="review-action-menu review-source-menu" align="start" sideOffset={6}>
          <DropdownMenu.RadioGroup value={value} onValueChange={onChange} aria-label={label}>
            {options.map((option) => (
              <DropdownMenu.RadioItem key={option.value} value={option.value} textValue={option.label}>
                <span>{option.label}</span>
                <DropdownMenu.ItemIndicator><Check size={16} aria-hidden="true" /></DropdownMenu.ItemIndicator>
              </DropdownMenu.RadioItem>
            ))}
          </DropdownMenu.RadioGroup>
        </DropdownMenu.Content>
      </DropdownMenu.Portal>
    </DropdownMenu.Root>
  );
}
