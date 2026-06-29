import { Building2, Upload } from "lucide-react";
import {
  Button,
  Card,
  CardContent,
  Table,
  Tbody,
  Td,
  Th,
  Thead,
  Tr,
} from "@/components/ui";
import { FyiMcpClient, type FyiAuthority } from "@/lib/mcp-client";

export const dynamic = "force-dynamic";

async function getAuthorities(): Promise<{ authorities: FyiAuthority[]; error?: string }> {
  const client = new FyiMcpClient();

  try {
    return { authorities: await client.listAuthorities() };
  } catch (error) {
    return {
      authorities: [],
      error: error instanceof Error ? error.message : "Unable to load authorities",
    };
  } finally {
    await client.close();
  }
}

export default async function AuthoritiesPage() {
  const { authorities, error } = await getAuthorities();

  return (
    <div className="space-y-6">
      <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <h2 className="text-2xl font-bold text-gray-900 dark:text-gray-100">
            Authorities
          </h2>
          <p className="mt-1 text-sm text-gray-500 dark:text-gray-400">
            Browse public authorities available for tracked OIA requests
          </p>
        </div>
        <Button href="/authorities/import" variant="outline">
          <Upload className="h-4 w-4" />
          Import CSV
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
          {authorities.length > 0 ? (
            <Table>
              <Thead>
                <Tr>
                  <Th>Name</Th>
                  <Th>Slug</Th>
                  <Th>Source</Th>
                </Tr>
              </Thead>
              <Tbody>
                {authorities.map((authority) => (
                  <Tr key={authority.slug}>
                    <Td>
                      <span className="font-medium text-gray-900 dark:text-gray-100">
                        {authority.name}
                      </span>
                    </Td>
                    <Td>{authority.slug}</Td>
                    <Td>
                      {authority.url ? (
                        <a
                          href={authority.url}
                          className="text-brand-600 hover:text-brand-700 dark:text-brand-400 dark:hover:text-brand-300"
                          target="_blank"
                          rel="noreferrer"
                        >
                          Open
                        </a>
                      ) : (
                        "Not recorded"
                      )}
                    </Td>
                  </Tr>
                ))}
              </Tbody>
            </Table>
          ) : (
            <div className="flex min-h-72 flex-col items-center justify-center px-6 py-12 text-center">
              <div className="rounded-lg bg-gray-100 p-3 text-gray-500 dark:bg-gray-800 dark:text-gray-400">
                <Building2 className="h-6 w-6" />
              </div>
              <h3 className="mt-4 text-sm font-semibold text-gray-900 dark:text-gray-100">
                No authorities found
              </h3>
              <p className="mt-1 max-w-sm text-sm text-gray-500 dark:text-gray-400">
                Imported authorities will appear here and become available in the request creation form.
              </p>
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
