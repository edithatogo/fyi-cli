"use client";

import {
  Bar,
  BarChart,
  CartesianGrid,
  Line,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import { Card, CardContent, CardHeader } from "@/components/ui";
import type { DashboardChartData } from "@/lib/dashboard-summary";

type RequestTimelineDatum = DashboardChartData["requestTimeline"][number];

interface RequestTimelineChartProps {
  data: RequestTimelineDatum[];
}

function formatMonth(month: string) {
  if (month === "Unknown") {
    return month;
  }

  const [year, monthNumber] = month.split("-");
  const date = new Date(Number(year), Number(monthNumber) - 1, 1);
  return new Intl.DateTimeFormat("en", {
    month: "short",
    year: "numeric",
  }).format(date);
}

export function RequestTimelineChart({ data }: RequestTimelineChartProps) {
  const chartData = data.map((entry) => ({
    ...entry,
    label: formatMonth(entry.month),
  }));

  return (
    <Card>
      <CardHeader>
        <h3 className="text-base font-semibold text-gray-900 dark:text-gray-100">
          Request timeline
        </h3>
      </CardHeader>
      <CardContent>
        {chartData.length > 0 ? (
          <div className="grid gap-6 lg:grid-cols-[minmax(0,1fr)_180px]">
            <div className="h-64 min-w-0">
              <ResponsiveContainer width="100%" height="100%">
                <BarChart data={chartData} margin={{ left: -20, right: 8 }}>
                  <CartesianGrid strokeDasharray="3 3" vertical={false} />
                  <XAxis dataKey="label" tickLine={false} axisLine={false} />
                  <YAxis allowDecimals={false} tickLine={false} axisLine={false} />
                  <Tooltip
                    formatter={(value) => [value, "Requests"]}
                    labelFormatter={(label) => String(label)}
                  />
                  <Bar
                    dataKey="requests"
                    fill="#2563eb"
                    radius={[6, 6, 0, 0]}
                  />
                  <Line
                    type="monotone"
                    dataKey="requests"
                    stroke="#059669"
                    strokeWidth={2}
                    dot={{ r: 3 }}
                  />
                </BarChart>
              </ResponsiveContainer>
            </div>

            <div className="grid content-center gap-3">
              {chartData.map((entry) => (
                <div
                  key={entry.month}
                  className="flex items-center justify-between gap-3 text-sm"
                >
                  <span className="truncate text-gray-600 dark:text-gray-400">
                    {entry.label}
                  </span>
                  <span className="font-semibold text-gray-900 dark:text-gray-100">
                    {entry.requests}
                  </span>
                </div>
              ))}
            </div>
          </div>
        ) : (
          <div className="flex h-64 items-center justify-center rounded-lg border border-dashed border-gray-300 text-sm text-gray-500 dark:border-gray-700 dark:text-gray-400">
            No request timeline data
          </div>
        )}
      </CardContent>
    </Card>
  );
}
