import { useContext } from "react";
import { LocaleContext } from "./locale-context";

export function useLocale() {
  return useContext(LocaleContext);
}
