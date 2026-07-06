import type { PlanItem, Reminder } from "./commands";
import { reminderDateKey } from "./dateKeys";

export interface CalendarMarks {
  tasks: number;
  reminders: number;
}

export function buildCalendarMarks(items: PlanItem[], reminders: Reminder[]): Record<string, CalendarMarks> {
  const marks: Record<string, CalendarMarks> = {};
  for (const item of items) {
    const mark = marks[item.planDate] ?? { tasks: 0, reminders: 0 };
    mark.tasks += 1;
    marks[item.planDate] = mark;
  }
  for (const reminder of reminders) {
    const key = reminderDateKey(reminder.triggerAtUtc);
    const mark = marks[key] ?? { tasks: 0, reminders: 0 };
    mark.reminders += 1;
    marks[key] = mark;
  }
  return marks;
}
