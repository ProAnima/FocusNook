import { invoke } from "@tauri-apps/api/core";
import type { PlanItem } from "./types";

export const planItemsCommands = {
  async list(planDate: string): Promise<PlanItem[]> {
    return invoke<PlanItem[]>("list_plan_items", { planDate });
  },
  async listRange(startDate: string, endDate: string): Promise<PlanItem[]> {
    return invoke<PlanItem[]>("list_plan_items_range", { startDate, endDate });
  },
  async create(title: string, planDate: string): Promise<PlanItem> {
    return invoke<PlanItem>("create_plan_item", { title, planDate });
  },
  async toggleDone(id: string): Promise<PlanItem> {
    return invoke<PlanItem>("toggle_plan_item_done", { id });
  },
  async cycleProgress(id: string): Promise<PlanItem> {
    return invoke<PlanItem>("cycle_plan_item_progress", { id });
  },
  async toggleDeferred(id: string): Promise<PlanItem> {
    return invoke<PlanItem>("toggle_plan_item_deferred", { id });
  },
  async toggleLongRunning(id: string): Promise<PlanItem> {
    return invoke<PlanItem>("toggle_plan_item_long_running", { id });
  },
  async moveToDate(id: string, planDate: string): Promise<PlanItem> {
    return invoke<PlanItem>("move_plan_item_to_date", { id, planDate });
  },
  async rollOverPending(targetDate: string): Promise<number> {
    return invoke<number>("roll_over_pending_plan_items", { targetDate });
  },
  async delete(id: string) {
    await invoke("delete_plan_item", { id });
  },
};
