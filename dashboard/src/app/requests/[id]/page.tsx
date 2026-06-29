import { notFound, redirect } from "next/navigation";
import { ArrowLeft, Save, Trash2 } from "lucide-react";
import {
  Badge,
  Button,
  Card,
  CardContent,
  CardHeader,
  Input,
  Select,
  Table,
  Tbody,
  Td,
  Th,
  Thead,
  Tr,
} from "@/components/ui";
import { FyiMcpClient, type FyiRequestWithCorrespondence } from "@/lib/mcp-client";

export const dynamic = "force-dynamic";

const statusOptions = [
  { value: "draft", label: "Draft" },
  { value: "submitted", label: "Submitted" },
  { value: "awaiting_response", label: "Awaiting response" },
  { value: "partial", label: "Partial response" },
  { value: "completed", label: "Completed" },
  { value: "closed", label: "Closed" },
];

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

async function updateRequest(formData: FormData) {
  "use server";

  const id = Number(formData.get("id"));
  const title = formData.get("title")?.toString().trim();
  const body = formData.get("body")?.toString().trim();
  const userName = formData.get("user_name")?.toString().trim();
  const status = formData.get("status")?.toString().trim();
  const url = formData.get("url")?.toString().trim();
  const tags = formData
    .get("tags")
    ?.toString()
    .split(",")
    .map((tag) => tag.trim())
    .filter(Boolean) ?? [];

  if (!Number.isFinite(id) || !title || !body) {
    throw new Error("Request ID, title, and body are required.");
  }

  const client = new FyiMcpClient();
  try {
    await client.updateRequest({
      id,
      title,
      body,
      user_name: userName || undefined,
      status: status || undefined,
      url: url || undefined,
      tags,
    });
  } finally {
    await client.close();
  }

  redirect(`/requests/${id}`);
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

      <Card>
        <CardHeader>
          <h3 className="text-base font-semibold text-gray-900 dark:text-gray-100">
            Inline edit
          </h3>
        </CardHeader>
        <CardContent>
          <form action={updateRequest} className="grid gap-5">
            <input type="hidden" name="id" value={request.id} />
            <label className="grid gap-2">
              <span className="text-sm font-medium text-gray-700 dark:text-gray-300">
                Title
              </span>
              <Input name="title" required defaultValue={request.title} />
            </label>

            <label className="grid gap-2">
              <span className="text-sm font-medium text-gray-700 dark:text-gray-300">
                Body
              </span>
              <textarea
                name="body"
                required
                rows={10}
                defaultValue={request.body}
                className="w-full rounded-lg border border-gray-300 bg-white px-3 py-2 text-sm text-gray-900 placeholder:text-gray-400 focus-visible:border-brand-500 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-brand-500 dark:border-gray-600 dark:bg-gray-900 dark:text-gray-100 dark:placeholder:text-gray-500"
              />
            </label>

            <div className="grid gap-5 md:grid-cols-2">
              <label className="grid gap-2">
                <span className="text-sm font-medium text-gray-700 dark:text-gray-300">
                  Status
                </span>
                <Select
                  name="status"
                  options={statusOptions}
                  defaultValue={request.status ?? "draft"}
                />
              </label>

              <label className="grid gap-2">
                <span className="text-sm font-medium text-gray-700 dark:text-gray-300">
                  Requester
                </span>
                <Input name="user_name" defaultValue={request.user_name ?? ""} />
              </label>
            </div>

            <div className="grid gap-5 md:grid-cols-2">
              <label className="grid gap-2">
                <span className="text-sm font-medium text-gray-700 dark:text-gray-300">
                  Source URL
                </span>
                <Input name="url" defaultValue={request.url ?? ""} />
              </label>

              <label className="grid gap-2">
                <span className="text-sm font-medium text-gray-700 dark:text-gray-300">
                  Tags
                </span>
                <Input name="tags" defaultValue={request.tags?.join(", ") ?? ""} />
              </label>
            </div>

            <div className="flex justify-end border-t border-gray-200 pt-5 dark:border-gray-800">
              <Button type="submit">
                <Save className="h-4 w-4" />
                Save changes
              </Button>
            </div>
          </form>
        </CardContent>
      </Card>

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

      <Card>
        <CardHeader>
          <h3 className="text-base font-semibold text-gray-900 dark:text-gray-100">
            Correspondence
          </h3>
        </CardHeader>
        <CardContent className="p-0">
          {correspondence.length > 0 ? (
            <Table>
              <Thead>
                <Tr>
                  <Th>Direction</Th>
                  <Th>Sent</Th>
                  <Th>State</Th>
                  <Th>Body</Th>
                </Tr>
              </Thead>
              <Tbody>
                {correspondence.map((item, index) => (
                  <Tr key={`${item.sent_at}-${index}`}>
                    <Td>{item.direction}</Td>
                    <Td>{item.sent_at}</Td>
                    <Td>{item.state ?? "Not recorded"}</Td>
                    <Td>{item.body}</Td>
                  </Tr>
                ))}
              </Tbody>
            </Table>
          ) : (
            <div className="px-6 py-10 text-sm text-gray-500 dark:text-gray-400">
              No correspondence has been captured for this request yet.
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
