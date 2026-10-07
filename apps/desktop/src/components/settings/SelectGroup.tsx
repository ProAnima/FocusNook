import { useRef, useState, type ReactNode } from "react";
import { Check, ChevronDown } from "lucide-react";
import { useOutsideClick } from "../../shared/useOutsideClick";

export interface SelectOption<T extends string | null> {
  value: T;
  label: string;
}

interface SelectGroupProps<T extends string | null> {
  label: string;
  options: SelectOption<T>[];
  selected: T;
  selectedLabel: string;
  onSelect: (value: T) => void;
  /** Иконка слева от подписи выбранного значения. */
  leadingIcon?: ReactNode;
  /** Кнопка рядом с триггером (например, «обновить список»). */
  trailing?: ReactNode;
  /** Дополнительное содержимое группы под выпадающим меню. */
  children?: ReactNode;
}

/** Группа настроек с выпадающим списком (listbox) — общая для языка и микрофона. */
export function SelectGroup<T extends string | null>({
  label,
  options,
  selected,
  selectedLabel,
  onSelect,
  leadingIcon,
  trailing,
  children,
}: SelectGroupProps<T>) {
  const [open, setOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);
  useOutsideClick(rootRef, () => setOpen(false));

  const trigger = (
    <button
      className={`settings-select-trigger ${open ? "is-open" : ""}`}
      type="button"
      onClick={() => setOpen((value) => !value)}
      aria-label={label}
      aria-haspopup="listbox"
      aria-expanded={open}
    >
      {leadingIcon}
      <span>{selectedLabel}</span>
      <ChevronDown size={15} />
    </button>
  );

  return (
    <div className="settings-group" ref={rootRef}>
      <span className="settings-group-label">{label}</span>
      {trailing ? <div className="settings-inline-row">{trigger}{trailing}</div> : trigger}
      {open && (
        <div className="settings-select-menu" role="listbox" aria-label={label}>
          {options.map((option) => (
            <SelectOptionButton
              key={option.value ?? "__default"}
              option={option}
              active={option.value === selected}
              onPick={() => {
                setOpen(false);
                onSelect(option.value);
              }}
            />
          ))}
        </div>
      )}
      {children}
    </div>
  );
}

function SelectOptionButton<T extends string | null>({
  option,
  active,
  onPick,
}: {
  option: SelectOption<T>;
  active: boolean;
  onPick: () => void;
}) {
  return (
    <button
      className={`settings-select-option ${active ? "is-active" : ""}`}
      type="button"
      role="option"
      aria-selected={active}
      onClick={onPick}
    >
      <span>{option.label}</span>
      {active && <Check size={13} />}
    </button>
  );
}
