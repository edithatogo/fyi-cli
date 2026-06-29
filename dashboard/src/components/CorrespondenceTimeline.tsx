import { Mail, MessageSquare, Paperclip } from "lucide-react";
import { Badge, Card, CardContent, CardHeader } from "@/components/ui";
import type { FyiCorrespondence } from "@/lib/mcp-client";

interface CorrespondenceTimelineProps {
  correspondence: FyiCorrespondence[];
}

function formatDate(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }

  return new Intl.DateTimeFormat("en-NZ", {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(date);
}

function eventTitle(direction: string) {
  return direction.toLowerCase() === "request"
    ? "Request sent"
    : "Response received";
}

function badgeVariant(direction: string) {
  return direction.toLowerCase() === "request" ? "info" : "success";
}

export function CorrespondenceTimeline({
  correspondence,
}: CorrespondenceTimelineProps) {
  const events = [...correspondence].sort(
    (left, right) =>
      new Date(left.sent_at).getTime() - new Date(right.sent_at).getTime()
  );

  return (
    <Card>
      <CardHeader>
        <h3 className="text-base font-semibold text-gray-900 dark:text-gray-100">
          Correspondence timeline
        </h3>
      </CardHeader>
      <CardContent>
        {events.length > 0 ? (
          <ol className="relative space-y-6 border-l border-gray-200 pl-6 dark:border-gray-800">
            {events.map((item, index) => {
              const direction = item.direction.toLowerCase();
              const Icon = direction === "request" ? Mail : MessageSquare;
              const attachments = item.attachments ?? [];

              return (
                <li key={`${item.sent_at}-${index}`} className="relative">
                  <span className="absolute -left-[35px] flex h-7 w-7 items-center justify-center rounded-full border border-gray-200 bg-white text-brand-600 shadow-sm dark:border-gray-700 dark:bg-gray-900 dark:text-brand-300">
                    <Icon className="h-4 w-4" />
                  </span>
                  <div className="grid gap-3 rounded-lg border border-gray-200 bg-gray-50 p-4 dark:border-gray-800 dark:bg-gray-950/40">
                    <div className="flex flex-col gap-2 sm:flex-row sm:items-start sm:justify-between">
                      <div>
                        <p className="font-medium text-gray-900 dark:text-gray-100">
                          {eventTitle(item.direction)}
                        </p>
                        <p className="mt-1 text-sm text-gray-500 dark:text-gray-400">
                          {formatDate(item.sent_at)}
                        </p>
                      </div>
                      <div className="flex flex-wrap gap-2">
                        <Badge variant={badgeVariant(item.direction)}>
                          {item.state ?? item.direction}
                        </Badge>
                        {attachments.length > 0 && (
                          <Badge variant="default">
                            <Paperclip className="mr-1 h-3 w-3" />
                            {attachments.length}{" "}
                            {attachments.length === 1 ? "attachment" : "attachments"}
                          </Badge>
                        )}
                      </div>
                    </div>
                    <p className="whitespace-pre-wrap text-sm leading-6 text-gray-700 dark:text-gray-300">
                      {item.body}
                    </p>
                  </div>
                </li>
              );
            })}
          </ol>
        ) : (
          <div className="rounded-lg border border-dashed border-gray-300 px-6 py-10 text-sm text-gray-500 dark:border-gray-700 dark:text-gray-400">
            No correspondence has been captured for this request yet.
          </div>
        )}
      </CardContent>
    </Card>
  );
}
