"use client";

import {
  Cell,
  Pie,
  PieChart,
  ResponsiveContainer,
  Tooltip,
} from "recharts";
import { Card, CardContent, CardHeader } from "@/components/ui";
import type { DashboardChartData } from "@/lib/dashboard-summary";

type StatusDistributionDatum = DashboardChartData["statusDistribution"][number];

interface StatusDistributionChartProps {
  data: StatusDistributionDatum[];
}

const STATUS_COLORS = [
  "#2563eb",
  "#059669",
  "#d97706",
  "#dc2626",
  "#7c3aed",
  "#475569",
];

function formatStatus(status: string) {
  return status
    .split(/[_\s-]+/)
    .filter(Boolean)
    .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
    .join(" ");
}

export function StatusDistributionChart({ data }: StatusDistributionChartProps) {
  return (
    <Card>
      <CardHeader>
        <h3 className="text-base font-semibold text-gray-900 dark:text-gray-100">
          Status distribution
        </h3>
      </CardHeader>
      <CardContent>
        {data.length > 0 ? (
          <div className="grid gap-6 lg:grid-cols-[minmax(0,1fr)_180px]">
            <div className="h-64 min-w-0">
              <ResponsiveContainer width="100%" height="100%">
                <PieChart>
                  <Pie
                    data={data}
                    dataKey="count"
                    nameKey="status"
                    innerRadius="58%"
                    outerRadius="82%"
                    paddingAngle={2}
                  >
                    {data.map((entry, index) => (
                      <Cell
                        key={entry.status}
                        fill={STATUS_COLORS[index % STATUS_COLORS.length]}
                      />
                    ))}
                  </Pie>
                  <Tooltip
                    formatter={(value, name) => [
                      value,
                      formatStatus(String(name)),
                    ]}
                  />
                </PieChart>
              </ResponsiveContainer>
            </div>

            <div className="grid content-center gap-3">
              {data.map((entry, index) => (
                <div
                  key={entry.status}
                  className="flex items-center justify-between gap-3 text-sm"
                >
                  <span className="flex min-w-0 items-center gap-2 text-gray-600 dark:text-gray-400">
                    <span
                      className="h-2.5 w-2.5 shrink-0 rounded-full"
                      style={{
                        backgroundColor:
                          STATUS_COLORS[index % STATUS_COLORS.length],
                      }}
                    />
                    <span className="truncate">{formatStatus(entry.status)}</span>
                  </span>
                  <span className="font-semibold text-gray-900 dark:text-gray-100">
                    {entry.count}
                  </span>
                </div>
              ))}
            </div>
          </div>
        ) : (
          <div className="flex h-64 items-center justify-center rounded-lg border border-dashed border-gray-300 text-sm text-gray-500 dark:border-gray-700 dark:text-gray-400">
            No request status data
          </div>
        )}
      </CardContent>
    </Card>
  );
}
