import { notFound, redirect } from "next/navigation";
import { ArrowLeft, Trash2 } from "lucide-react";
import { CorrespondenceTimeline } from "@/components/CorrespondenceTimeline";
import { RequestInlineEditor } from "@/components/RequestInlineEditor";
import {
  Badge,
  Button,
  Card,
  CardContent,
  CardHeader,
} from "@/components/ui";
import { FyiMcpClient, type FyiRequestWithCorrespondence } from "@/lib/mcp-client";

export const dynamic = "force-dynamic";

async function getRequest(id: number): Promise<FyiRequestWithCorrespondence> {
  const client = new FyiMcpClient();

  try {
    return await client.retrieveRequest(id);
  } catch {
    notFound();
  } finally {
    await client.close();
  }
}

async function deleteRequest(formData: FormData) {
  "use server";

  const id = Number(formData.get("id"));
  const confirmed = formData.get("confirm_delete") === "on";

  if (!Number.isFinite(id) || !confirmed) {
    throw new Error("Confirm deletion before deleting this request.");
  }

  const client = new FyiMcpClient();
  try {
    await client.deleteRequest(id);
  } finally {
    await client.close();
  }

  redirect("/requests");
}

function badgeVariant(status?: string | null) {
  switch (status) {
    case "completed":
    case "successful":
      return "success";
    case "awaiting_response":
    case "waiting_response":
    case "submitted":
      return "warning";
    case "rejected":
    case "overdue":
      return "danger";
    default:
      return "default";
  }
}

export default async function RequestDetailPage({
  params,
}: {
  params: { id: string };
}) {
  const id = Number(params.id);
  if (!Number.isFinite(id)) {
    notFound();
  }

  const { request, correspondence } = await getRequest(id);

  return (
    <div className="space-y-6">
      <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <div className="flex flex-wrap items-center gap-3">
            <h2 className="text-2xl font-bold text-gray-900 dark:text-gray-100">
              {request.title}
            </h2>
            <Badge variant={badgeVariant(request.status)}>
              {request.status ?? "unknown"}
            </Badge>
          </div>
          <p className="mt-1 text-sm text-gray-500 dark:text-gray-400">
            Request #{request.id}
          </p>
        </div>
        <Button href="/requests" variant="outline">
          <ArrowLeft className="h-4 w-4" />
          Back
        </Button>
      </div>

      <RequestInlineEditor request={request} />

      <Card className="border-red-200 dark:border-red-900">
        <CardHeader>
          <h3 className="text-base font-semibold text-red-700 dark:text-red-300">
            Delete request
          </h3>
        </CardHeader>
        <CardContent>
          <form action={deleteRequest} className="grid gap-4">
            <input type="hidden" name="id" value={request.id} />
            <label className="flex items-start gap-3 text-sm text-gray-600 dark:text-gray-300">
              <input
                name="confirm_delete"
                type="checkbox"
                className="mt-1 h-4 w-4 rounded border-gray-300 text-red-600 focus:ring-red-500"
                required
              />
              <span>
                I understand this will delete the request and any captured correspondence from the local database.
              </span>
            </label>
            <div className="flex justify-end">
              <Button type="submit" variant="danger">
                <Trash2 className="h-4 w-4" />
                Delete request
              </Button>
            </div>
          </form>
        </CardContent>
      </Card>

      <CorrespondenceTimeline request={request} correspondence={correspondence} />
    </div>
  );
}
