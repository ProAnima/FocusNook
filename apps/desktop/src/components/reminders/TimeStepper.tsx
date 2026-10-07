import { ChevronDown, ChevronUp } from "lucide-react";
import { pad, wrapTimeValue } from "./reminderTime";

interface TimeStepperProps {
  label: string;
  value: number;
  min: number;
  max: number;
  step?: number;
  onChange: (value: number) => void;
}

export function TimeStepper({ label, value, min, max, step = 1, onChange }: TimeStepperProps) {
  const upTitle = `${label} +`;
  const downTitle = `${label} -`;
  return (
    <div className="time-stepper" aria-label={label}>
      <button type="button" onClick={() => onChange(wrapTimeValue(value + step, min, max))} title={upTitle} aria-label={upTitle}>
        <ChevronUp size={12} />
      </button>
      <span>{pad(value)}</span>
      <button type="button" onClick={() => onChange(wrapTimeValue(value - step, min, max))} title={downTitle} aria-label={downTitle}>
        <ChevronDown size={12} />
      </button>
    </div>
  );
}
