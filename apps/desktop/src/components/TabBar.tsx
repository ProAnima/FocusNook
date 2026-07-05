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
}: {
  tabs: readonly TabDefinition<K>[];
  active: K;
  onSelect: (key: K) => void;
}) {
  return (
    <nav className="tabs">
      {tabs.map(({ key, label, icon: Icon }) => (
        <button
          key={key}
          className={`tab ${active === key ? "tab-active" : ""}`}
          onClick={() => onSelect(key)}
        >
          <Icon size={13} />
          <span>{label}</span>
        </button>
      ))}
    </nav>
  );
}
