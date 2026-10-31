export interface ScheduleTrack {
  /** Key used to look up a day's recipe in the plan. */
  key: string;
  label: string;
  /** Mantine color name for the row label. */
  color: string;
}

/** Schedule rows, top to bottom. Add, remove or reorder entries to change the grid. */
export const scheduleTracks: ScheduleTrack[] = [
  { key: "breakfast", label: "Breakfast", color: "yellow" },
  { key: "breakfast-kids", label: "Breakfast Kids", color: "yellow" },
  { key: "lunch", label: "Lunch", color: "myColor" },
  { key: "dinner", label: "Dinner", color: "blue" },
];
