import type { LucideIcon } from "lucide-react";

export interface TabDefinition<K extends string> {
  key: K;
  label: string;
  icon: LucideIcon;
}

export function TabBar<K extends string>({
  tabs,
  active,
  onSelect,
  className,
  iconSize = 13,
}: {
  tabs: readonly TabDefinition<K>[];
  active: K;
  onSelect: (key: K) => void;
  className?: string;
  iconSize?: number;
}) {
  return (
    <nav className={`tabs ${className ?? ""}`}>
      {tabs.map(({ key, label, icon: Icon }) => (
        <button
          key={key}
          className={`tab ${active === key ? "tab-active" : ""}`}
          onClick={() => onSelect(key)}
        >
          <Icon size={iconSize} />
          <span>{label}</span>
        </button>
      ))}
    </nav>
  );
}
