import { Clock, Mail, MessageSquare, Paperclip } from "lucide-react";
import { Badge, Card, CardContent, CardHeader } from "@/components/ui";
import type { FyiCorrespondence, FyiRequest } from "@/lib/mcp-client";

interface CorrespondenceTimelineProps {
  request?: FyiRequest;
  correspondence: FyiCorrespondence[];
}

type TimelineEvent =
  | {
      type: "created" | "updated";
      title: string;
      timestamp: string;
      badge: string;
      body: string;
    }
  | {
      type: "correspondence";
      timestamp: string;
      correspondence: FyiCorrespondence;
    };

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

function correspondenceTitle(direction: string) {
  return direction.toLowerCase() === "request"
    ? "Request sent"
    : "Response received";
}

function directionBadgeVariant(direction: string) {
  return direction.toLowerCase() === "request" ? "info" : "success";
}

function buildTimelineEvents(
  request: FyiRequest | undefined,
  correspondence: FyiCorrespondence[]
) {
  const events: TimelineEvent[] = correspondence.map((item) => ({
    type: "correspondence",
    timestamp: item.sent_at,
    correspondence: item,
  }));

  if (request?.created_at) {
    events.push({
      type: "created",
      title: "Request created",
      timestamp: request.created_at,
      badge: request.status ?? "created",
      body: request.title,
    });
  }

  if (request?.updated_at && request.updated_at !== request.created_at) {
    events.push({
      type: "updated",
      title: "Request updated",
      timestamp: request.updated_at,
      badge: request.status ?? "updated",
      body: request.title,
    });
  }

  return events.sort((left, right) => {
    const leftTime = new Date(left.timestamp).getTime();
    const rightTime = new Date(right.timestamp).getTime();
    if (Number.isNaN(leftTime) || Number.isNaN(rightTime)) {
      return left.timestamp.localeCompare(right.timestamp);
    }
    return leftTime - rightTime;
  });
}

export function CorrespondenceTimeline({
  request,
  correspondence,
}: CorrespondenceTimelineProps) {
  const events = buildTimelineEvents(request, correspondence);

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
              if (item.type !== "correspondence") {
                return (
                  <li key={`${item.timestamp}-${item.type}`} className="relative">
                    <span className="absolute -left-[35px] flex h-7 w-7 items-center justify-center rounded-full border border-gray-200 bg-white text-brand-600 shadow-sm dark:border-gray-700 dark:bg-gray-900 dark:text-brand-300">
                      <Clock className="h-4 w-4" />
                    </span>
                    <div className="grid gap-3 rounded-lg border border-gray-200 bg-gray-50 p-4 dark:border-gray-800 dark:bg-gray-950/40">
                      <div className="flex flex-col gap-2 sm:flex-row sm:items-start sm:justify-between">
                        <div>
                          <p className="font-medium text-gray-900 dark:text-gray-100">
                            {item.title}
                          </p>
                          <p className="mt-1 text-sm text-gray-500 dark:text-gray-400">
                            {formatDate(item.timestamp)}
                          </p>
                        </div>
                        <div className="flex flex-wrap gap-2">
                          <Badge variant="warning">{item.badge}</Badge>
                        </div>
                      </div>
                      <p className="text-sm leading-6 text-gray-700 dark:text-gray-300">
                        {item.body}
                      </p>
                    </div>
                  </li>
                );
              }

              const correspondenceItem = item.correspondence;
              const direction = correspondenceItem.direction.toLowerCase();
              const Icon = direction === "request" ? Mail : MessageSquare;
              const attachments = correspondenceItem.attachments ?? [];

              return (
                <li key={`${item.timestamp}-${index}`} className="relative">
                  <span className="absolute -left-[35px] flex h-7 w-7 items-center justify-center rounded-full border border-gray-200 bg-white text-brand-600 shadow-sm dark:border-gray-700 dark:bg-gray-900 dark:text-brand-300">
                    <Icon className="h-4 w-4" />
                  </span>
                  <div className="grid gap-3 rounded-lg border border-gray-200 bg-gray-50 p-4 dark:border-gray-800 dark:bg-gray-950/40">
                    <div className="flex flex-col gap-2 sm:flex-row sm:items-start sm:justify-between">
                      <div>
                        <p className="font-medium text-gray-900 dark:text-gray-100">
                          {correspondenceTitle(correspondenceItem.direction)}
                        </p>
                        <p className="mt-1 text-sm text-gray-500 dark:text-gray-400">
                          {formatDate(item.timestamp)}
                        </p>
                      </div>
                      <div className="flex flex-wrap gap-2">
                        <Badge variant={directionBadgeVariant(correspondenceItem.direction)}>
                          {correspondenceItem.state ?? correspondenceItem.direction}
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
                      {correspondenceItem.body}
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
