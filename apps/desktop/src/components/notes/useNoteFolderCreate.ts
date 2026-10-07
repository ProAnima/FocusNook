import { useCallback, useState, type FormEvent } from "react";

export function useNoteFolderCreate(onCreate: (name: string) => void) {
  const [draft, setDraft] = useState("");
  const [creating, setCreating] = useState(false);

  const cancelCreate = useCallback(() => {
    setDraft("");
    setCreating(false);
  }, []);

  const submit = useCallback(
    (event: FormEvent) => {
      event.preventDefault();
      const name = draft.trim();
      if (!name) return;
      setDraft("");
      setCreating(false);
      onCreate(name);
    },
    [draft, onCreate],
  );

  const toggleCreating = useCallback(() => setCreating((value) => !value), []);

  return { draft, setDraft, creating, submit, cancelCreate, toggleCreating };
}
