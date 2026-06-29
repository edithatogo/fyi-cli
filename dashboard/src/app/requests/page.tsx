import { Plus } from "lucide-react";
import { RequestsTable } from "@/components/RequestsTable";
import { Button, Card, CardContent } from "@/components/ui";
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

      <RequestsTable requests={requests} />
    </div>
  );
}
