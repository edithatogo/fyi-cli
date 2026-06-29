import { FileText, Plus } from "lucide-react";
import { Badge, Button, Card, CardContent, Table, Tbody, Td, Th, Thead, Tr } from "@/components/ui";
import { FyiMcpClient, type FyiRequest } from "@/lib/mcp-client";

export const dynamic = "force-dynamic";

async function getRequests(): Promise<{ requests: FyiRequest[]; error?: string }> {
  const client = new FyiMcpClient();

  try {
    return { requests: await client.listRequests(100) };
  } catch (error) {
    return {
      requests: [],
      error: error instanceof Error ? error.message : "Unable to load requests",
    };
  } finally {
    await client.close();
  }
}

function statusVariant(status?: string | null) {
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

export default async function RequestsPage() {
  const { requests, error } = await getRequests();

  return (
    <div className="space-y-6">
      <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <h2 className="text-2xl font-bold text-gray-900 dark:text-gray-100">
            Requests
          </h2>
          <p className="mt-1 text-sm text-gray-500 dark:text-gray-400">
            Tracked OIA requests from the local FYI database
          </p>
        </div>
        <Button href="/requests/new">
          <Plus className="h-4 w-4" />
          New request
        </Button>
      </div>

      {error && (
        <Card>
          <CardContent className="p-6">
            <p className="text-sm font-medium text-red-600 dark:text-red-400">
              {error}
            </p>
            <p className="mt-1 text-sm text-gray-500 dark:text-gray-400">
              Start the MCP backend or set FYI_MCP_COMMAND to the fyi-mcp executable.
            </p>
          </CardContent>
        </Card>
      )}

      <Card>
        <CardContent className="p-0">
          {requests.length > 0 ? (
            <Table>
              <Thead>
                <Tr>
                  <Th>Title</Th>
                  <Th>Status</Th>
                  <Th>Requester</Th>
                  <Th>Updated</Th>
                </Tr>
              </Thead>
              <Tbody>
                {requests.map((request) => (
                  <Tr key={request.id}>
                    <Td>
                      <a
                        href={`/requests/${request.id}`}
                        className="font-medium text-gray-900 hover:text-brand-600 dark:text-gray-100 dark:hover:text-brand-400"
                      >
                        {request.title}
                      </a>
                      <p className="mt-1 line-clamp-1 max-w-2xl text-xs text-gray-500 dark:text-gray-400">
                        {request.body}
                      </p>
                    </Td>
                    <Td>
                      <Badge variant={statusVariant(request.status)}>
                        {request.status ?? "unknown"}
                      </Badge>
                    </Td>
                    <Td>{request.user_name ?? "Not recorded"}</Td>
                    <Td>{request.updated_at ?? request.created_at ?? "Not recorded"}</Td>
                  </Tr>
                ))}
              </Tbody>
            </Table>
          ) : (
            <div className="flex min-h-72 flex-col items-center justify-center px-6 py-12 text-center">
              <div className="rounded-lg bg-gray-100 p-3 text-gray-500 dark:bg-gray-800 dark:text-gray-400">
                <FileText className="h-6 w-6" />
              </div>
              <h3 className="mt-4 text-sm font-semibold text-gray-900 dark:text-gray-100">
                No requests found
              </h3>
              <p className="mt-1 max-w-sm text-sm text-gray-500 dark:text-gray-400">
                Requests will appear here after they are imported or created through the MCP backend.
              </p>
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
