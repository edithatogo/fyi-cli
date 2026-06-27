import {
  FileText,
  Clock,
  CheckCircle2,
  AlertTriangle,
} from "lucide-react";

interface KpiCardProps {
  title: string;
  value: string | number;
  icon: React.ReactNode;
  description?: string;
  trend?: { value: string; positive: boolean };
}

function KpiCard({ title, value, icon, description, trend }: KpiCardProps) {
  return (
    <div className="rounded-xl border border-gray-200 bg-white p-6 shadow-sm dark:border-gray-800 dark:bg-gray-900">
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
          {trend && (
            <p
              className={`text-xs font-medium ${
                trend.positive ? "text-green-600" : "text-red-600"
              }`}
            >
              {trend.value} from last month
            </p>
          )}
        </div>
        <div className="rounded-lg bg-brand-50 p-3 text-brand-600 dark:bg-brand-900/50 dark:text-brand-400">
          {icon}
        </div>
      </div>
    </div>
  );
}

export function DashboardSummary() {
  // Placeholder data — will be fetched from MCP server
  const kpis = [
    {
      title: "Total Requests",
      value: 142,
      icon: <FileText className="h-6 w-6" />,
      description: "All time OIA requests",
      trend: { value: "+12", positive: true },
    },
    {
      title: "Pending",
      value: 23,
      icon: <Clock className="h-6 w-6" />,
      description: "Awaiting response",
      trend: { value: "-5", positive: true },
    },
    {
      title: "Completed",
      value: 108,
      icon: <CheckCircle2 className="h-6 w-6" />,
      description: "Fully resolved",
      trend: { value: "+18", positive: true },
    },
    {
      title: "Overdue",
      value: 4,
      icon: <AlertTriangle className="h-6 w-6" />,
      description: "Past statutory deadline",
      trend: { value: "+1", positive: false },
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

      <div className="rounded-xl border border-gray-200 bg-white p-6 shadow-sm dark:border-gray-800 dark:bg-gray-900">
        <h3 className="text-lg font-semibold text-gray-900 dark:text-gray-100">
          Recent Activity
        </h3>
        <p className="mt-2 text-sm text-gray-400 dark:text-gray-500">
          Dashboard data will be populated when connected to the MCP backend.
        </p>
      </div>
    </div>
  );
}