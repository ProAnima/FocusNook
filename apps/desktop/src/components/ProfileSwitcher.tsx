import { useRef, useState, type FormEvent } from "react";
import type { Profile } from "../shared/commands";
import { useLocale } from "../shared/useLocale";
import { useOutsideClick } from "../shared/useOutsideClick";

function ProfileMenuItem({
  profile,
  active,
  onSelect,
}: {
  profile: Profile;
  active: boolean;
  onSelect: () => void;
}) {
  return (
    <button className={`profile-menu-item ${active ? "is-active" : ""}`} onClick={onSelect}>
      <span className="profile-menu-dot" style={{ background: profile.avatarColor }} />
      {profile.displayName}
    </button>
  );
}

function ProfileMenu({
  profiles,
  activeProfileId,
  onSwitch,
  onCreate,
}: {
  profiles: Profile[];
  activeProfileId: string | null;
  onSwitch: (id: string) => void;
  onCreate: (displayName: string) => void;
}) {
  const [newName, setNewName] = useState("");
  const { t } = useLocale();

  function handleSubmit(event: FormEvent) {
    event.preventDefault();
    const name = newName.trim();
    if (!name) return;
    setNewName("");
    onCreate(name);
  }

  return (
    <div className="profile-menu">
      {profiles.map((profile) => (
        <ProfileMenuItem
          key={profile.id}
          profile={profile}
          active={profile.id === activeProfileId}
          onSelect={() => onSwitch(profile.id)}
        />
      ))}
      <form className="profile-menu-new" onSubmit={handleSubmit}>
        <input
          placeholder={t("profile.newPlaceholder")}
          value={newName}
          onChange={(event) => setNewName(event.target.value)}
        />
      </form>
    </div>
  );
}

interface ProfileSwitcherProps {
  profiles: Profile[];
  activeProfileId: string | null;
  onSwitch: (id: string) => void;
  onCreate: (displayName: string) => void;
}

// Раздел 15 ТЗ: "активный профиль выбирается в верхней панели", "быстрый
// switch без переустановки" — реализовано как компактный аватар-переключатель
// с выпадающим списком, а не отдельный экран.
export function ProfileSwitcher({ profiles, activeProfileId, onSwitch, onCreate }: ProfileSwitcherProps) {
  const [open, setOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);
  const active = profiles.find((profile) => profile.id === activeProfileId);
  useOutsideClick(rootRef, () => setOpen(false));

  if (!active) return null;

  return (
    <div className="profile-switcher" ref={rootRef}>
      <button
        className="profile-avatar"
        style={{ background: active.avatarColor }}
        onClick={() => setOpen((value) => !value)}
        title={active.displayName}
        aria-label={active.displayName}
        aria-haspopup="true"
        aria-expanded={open}
      >
        {active.displayName.charAt(0).toUpperCase()}
      </button>
      {open && (
        <ProfileMenu
          profiles={profiles}
          activeProfileId={activeProfileId}
          onSwitch={(id) => { setOpen(false); onSwitch(id); }}
          onCreate={(name) => { setOpen(false); onCreate(name); }}
        />
      )}
    </div>
  );
}
