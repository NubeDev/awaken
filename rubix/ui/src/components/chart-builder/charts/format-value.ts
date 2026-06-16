import { formatSecsToHoursMinsSecs } from "@/lib/utils";

type UnitType = "currency" | "duration" | "plain";

const columnUnitMap: Record<string, UnitType> = {
  cost: "currency",
  total_cost: "currency",
  input_cost: "currency",
  output_cost: "currency",
  duration: "duration",
};

const getUnitForColumn = (columnName?: string): UnitType => {
  if (!columnName) return "plain";

  // Direct match
  if (columnUnitMap[columnName]) return columnUnitMap[columnName];

  // Check if column name contains a known suffix
  const lowerName = columnName.toLowerCase();
  if (lowerName.includes("cost")) return "currency";
  if (lowerName.includes("duration")) return "duration";

  return "plain";
};

// `unitLabel` is the display label for a physical quantity the backend already
// converted to the user's unit system (§2) — e.g. "°F" / "°C". When given, it is
// appended to the (locale-formatted) number so a converted axis reads with its
// unit. Currency/duration columns keep their own formatting and ignore it.
export const formatMetricValue = (value: number, columnName?: string, unitLabel?: string): string => {
  const unit = getUnitForColumn(columnName);

  switch (unit) {
    case "currency":
      return `$${value.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`;
    case "duration":
      return formatSecsToHoursMinsSecs(value);
    case "plain":
    default: {
      const formatted = value.toLocaleString();
      return unitLabel ? `${formatted} ${unitLabel}` : formatted;
    }
  }
};
