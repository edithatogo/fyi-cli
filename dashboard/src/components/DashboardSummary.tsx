import {
  Building2,
  FileText,
  Clock,
  AlertTriangle,
} from "lucide-react";
import { AttentionTrendChart } from "@/components/AttentionTrendChart";
import { RequestTimelineChart } from "@/components/RequestTimelineChart";
import { StatusDistributionChart } from "@/components/StatusDistributionChart";
import { Card, CardContent } from "@/components/ui";
import type {
  DashboardChartData,
  DashboardSummaryData,
} from "@/lib/dashboard-summary";

interface KpiCardProps {
  title: string;
  value: string | number;
  icon: React.ReactNode;
  description?: string;
  tone?: "default" | "warning" | "danger";
}

function KpiCard({ title, value, icon, description, tone = "default" }: KpiCardProps) {
  const iconStyles = {
    default: "bg-brand-50 text-brand-600 dark:bg-brand-900/50 dark:text-brand-400",
    warning: "bg-amber-50 text-amber-600 dark:bg-amber-950/50 dark:text-amber-300",
    danger: "bg-red-50 text-red-600 dark:bg-red-950/50 dark:text-red-300",
  }[tone];

  return (
    <Card>
      <CardContent className="p-6">
        <div className="flex items-start justify-between">
          <div className="space-y-1">
            <p className="text-sm font-medium text-gray-500 dark:text-gray-400">
              {title}
            </p>
            <p className="text-3xl font-bold text-gray-900 dark:text-gray-100">
              {value}
            </p>
            {description && (
              <p className="text-xs text-gray-400 dark:text-gray-500">
                {description}
              </p>
            )}
          </div>
          <div className={`rounded-lg p-3 ${iconStyles}`}>
            {icon}
          </div>
        </div>
      </CardContent>
    </Card>
  );
}

interface DashboardSummaryProps {
  summary?: DashboardSummaryData;
  charts?: DashboardChartData;
  error?: string;
}

export function DashboardSummary({ summary, charts, error }: DashboardSummaryProps) {
  const data = summary ?? {
    totalRequests: 0,
    attentionNeeded: 0,
    overdue: 0,
    authoritiesCount: 0,
  };

  const kpis = [
    {
      title: "Total Requests",
      value: data.totalRequests,
      icon: <FileText className="h-6 w-6" />,
      description: "Tracked OIA requests",
    },
    {
      title: "Attention Needed",
      value: data.attentionNeeded,
      icon: <Clock className="h-6 w-6" />,
      description: "Needs operator review",
      tone: "warning" as const,
    },
    {
      title: "Overdue",
      value: data.overdue,
      icon: <AlertTriangle className="h-6 w-6" />,
      description: "Past statutory deadline",
      tone: "danger" as const,
    },
    {
      title: "Authorities",
      value: data.authoritiesCount,
      icon: <Building2 className="h-6 w-6" />,
      description: "Available request targets",
    },
  ];

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-2xl font-bold text-gray-900 dark:text-gray-100">
          Welcome to FYI
        </h2>
        <p className="mt-1 text-sm text-gray-500 dark:text-gray-400">
          Privacy-focused OIA request management
        </p>
      </div>

      <div className="grid gap-6 sm:grid-cols-2 lg:grid-cols-4">
        {kpis.map((kpi) => (
          <KpiCard key={kpi.title} {...kpi} />
        ))}
      </div>

      {charts && (
        <div className="grid gap-6 xl:grid-cols-2">
          <StatusDistributionChart data={charts.statusDistribution} />
          <RequestTimelineChart data={charts.requestTimeline} />
          <AttentionTrendChart data={charts.attentionTrend} />
        </div>
      )}

      {error && (
        <Card>
          <CardContent className="p-6">
            <p className="text-sm font-medium text-amber-700 dark:text-amber-300">
              Dashboard data could not be loaded.
            </p>
            <p className="mt-1 text-sm text-gray-500 dark:text-gray-400">
              {error}
            </p>
          </CardContent>
        </Card>
      )}

      <Card>
        <CardContent className="p-6">
          <h3 className="text-lg font-semibold text-gray-900 dark:text-gray-100">
            Recent Activity
          </h3>
          <p className="mt-2 text-sm text-gray-400 dark:text-gray-500">
            Dashboard data will be populated when connected to the MCP backend.
          </p>
        </CardContent>
      </Card>
    </div>
  );
}
