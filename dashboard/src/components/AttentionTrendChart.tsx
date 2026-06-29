"use client";

import {
  Area,
  AreaChart,
  CartesianGrid,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import { Card, CardContent, CardHeader } from "@/components/ui";
import type { DashboardChartData } from "@/lib/dashboard-summary";

type AttentionTrendDatum = DashboardChartData["attentionTrend"][number];

interface AttentionTrendChartProps {
  data: AttentionTrendDatum[];
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

export function AttentionTrendChart({ data }: AttentionTrendChartProps) {
  const chartData = data.map((entry) => ({
    ...entry,
    label: formatMonth(entry.month),
  }));

  return (
    <Card>
      <CardHeader>
        <h3 className="text-base font-semibold text-gray-900 dark:text-gray-100">
          Attention trend
        </h3>
      </CardHeader>
      <CardContent>
        {chartData.length > 0 ? (
          <div className="grid gap-6 lg:grid-cols-[minmax(0,1fr)_180px]">
            <div className="h-64 min-w-0">
              <ResponsiveContainer width="100%" height="100%">
                <AreaChart data={chartData} margin={{ left: -20, right: 8 }}>
                  <defs>
                    <linearGradient id="attentionTrendFill" x1="0" y1="0" x2="0" y2="1">
                      <stop offset="5%" stopColor="#d97706" stopOpacity={0.32} />
                      <stop offset="95%" stopColor="#d97706" stopOpacity={0.02} />
                    </linearGradient>
                  </defs>
                  <CartesianGrid strokeDasharray="3 3" vertical={false} />
                  <XAxis dataKey="label" tickLine={false} axisLine={false} />
                  <YAxis allowDecimals={false} tickLine={false} axisLine={false} />
                  <Tooltip
                    formatter={(value) => [value, "Attention needed"]}
                    labelFormatter={(label) => String(label)}
                  />
                  <Area
                    type="monotone"
                    dataKey="attentionNeeded"
                    stroke="#d97706"
                    strokeWidth={2}
                    fill="url(#attentionTrendFill)"
                  />
                </AreaChart>
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
                    {entry.attentionNeeded}
                  </span>
                </div>
              ))}
            </div>
          </div>
        ) : (
          <div className="flex h-64 items-center justify-center rounded-lg border border-dashed border-gray-300 text-sm text-gray-500 dark:border-gray-700 dark:text-gray-400">
            No attention trend data
          </div>
        )}
      </CardContent>
    </Card>
  );
}
