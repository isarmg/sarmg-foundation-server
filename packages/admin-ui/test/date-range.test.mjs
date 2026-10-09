import assert from "node:assert/strict";
import { test } from "node:test";
import { calendarDateErrors, isCalendarDate, isCalendarDateRange } from "../dist/DateRangeField.js";
test("dates use the Gregorian calendar without rollover or browser time zones", () => {
  for (const date of ["2024-02-29", "2000-02-29", "0001-01-01", "9999-12-31"]) assert.ok(isCalendarDate(date), date);
  for (const date of ["2023-02-29", "1900-02-29", "2022-04-31", "0000-01-01", "2022-2-01", " 2022-02-01", "2022-13-01"]) assert.equal(isCalendarDate(date), false, date);
  assert.deepEqual(calendarDateErrors(["2022", "2", "29"]), [false, false, true]);
  assert.deepEqual(calendarDateErrors(["2022", "13", "1"]), [false, true, false]);
  assert.deepEqual(calendarDateErrors(["year", "2", "1"]), [true, false, false]);
  assert.ok(isCalendarDateRange({ start: "2022-02-01", end: "2023-02-02" }));
  assert.ok(isCalendarDateRange({ start: "2022-02-01", end: "2022-02-01" }));
  assert.equal(isCalendarDateRange({ start: "2023-02-02", end: "2022-02-01" }), false);
});
