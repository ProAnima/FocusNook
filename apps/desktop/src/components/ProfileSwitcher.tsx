import { useRef, useState } from "react";
import { LogOut } from "lucide-react";
import type { Profile } from "../shared/commands";
import { useLocale } from "../shared/useLocale";
import { useOutsideClick } from "../shared/useOutsideClick";

export function ProfileSwitcher({ account, onLogout }: { account: Profile; onLogout: () => void }) {
  const [open, setOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);
  const { t } = useLocale();
  useOutsideClick(rootRef, () => setOpen(false));

  return (
    <div className="profile-switcher" ref={rootRef}>
      <button
        className="profile-avatar"
        style={{ background: account.avatarColor }}
        onClick={() => setOpen((value) => !value)}
        title={account.displayName}
        aria-label={account.displayName}
        aria-haspopup="true"
        aria-expanded={open}
      >
        {account.displayName.charAt(0).toUpperCase()}
      </button>
      {open && (
        <div className="profile-menu account-menu">
          <div className="account-menu-identity">
            <strong>{account.displayName}</strong>
            <span>{account.email}</span>
          </div>
          <button className="profile-menu-item" onClick={onLogout}>
            <LogOut size={14} />{t("account.logout")}
          </button>
        </div>
      )}
    </div>
  );
}
