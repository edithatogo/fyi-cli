import { redirect } from "next/navigation";
import { ArrowLeft, Upload } from "lucide-react";
import { Button, Card, CardContent, CardHeader, Input } from "@/components/ui";
import { FyiMcpClient, type FyiAuthority } from "@/lib/mcp-client";

export const dynamic = "force-dynamic";

type CsvColumn = "slug" | "name" | "url";

function splitCsvLine(line: string): string[] {
  const cells: string[] = [];
  let cell = "";
  let inQuotes = false;

  for (let index = 0; index < line.length; index += 1) {
    const character = line[index];
    const next = line[index + 1];

    if (character === '"' && inQuotes && next === '"') {
      cell += '"';
      index += 1;
      continue;
    }

    if (character === '"') {
      inQuotes = !inQuotes;
      continue;
    }

    if (character === "," && !inQuotes) {
      cells.push(cell.trim());
      cell = "";
      continue;
    }

    cell += character;
  }

  cells.push(cell.trim());
  return cells;
}

function parseAuthoritiesCsv(csv: string): FyiAuthority[] {
  const rows = csv
    .replace(/^\uFEFF/, "")
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean)
    .map(splitCsvLine);

  if (rows.length === 0) {
    throw new Error("CSV file is empty.");
  }

  const header = rows[0].map((column) => column.toLowerCase());
  const hasNamedColumns = header.includes("slug") && header.includes("name");
  const columnIndexes: Record<CsvColumn, number> = hasNamedColumns
    ? {
        slug: header.indexOf("slug"),
        name: header.indexOf("name"),
        url: header.indexOf("url"),
      }
    : {
        slug: 0,
        name: 1,
        url: 2,
      };
  const dataRows = hasNamedColumns ? rows.slice(1) : rows;

  const authorities = dataRows.map((row, index) => {
    const slug = row[columnIndexes.slug]?.trim() ?? "";
    const name = row[columnIndexes.name]?.trim() ?? "";
    const url =
      columnIndexes.url >= 0 ? row[columnIndexes.url]?.trim() || null : null;

    if (!slug || !name) {
      throw new Error(`CSV row ${index + (hasNamedColumns ? 2 : 1)} needs slug and name.`);
    }

    return { slug, name, url };
  });

  if (authorities.length === 0) {
    throw new Error("CSV file does not contain any authority rows.");
  }

  return authorities;
}

async function importAuthorities(formData: FormData) {
  "use server";

  const file = formData.get("file");
  if (!(file instanceof File) || file.size === 0) {
    throw new Error("Choose a CSV file to import.");
  }

  const authorities = parseAuthoritiesCsv(await file.text());
  const client = new FyiMcpClient();

  try {
    await client.importAuthorities(authorities);
  } finally {
    await client.close();
  }

  redirect("/authorities");
}

export default function ImportAuthoritiesPage() {
  return (
    <div className="space-y-6">
      <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <h2 className="text-2xl font-bold text-gray-900 dark:text-gray-100">
            Import authorities
          </h2>
          <p className="mt-1 text-sm text-gray-500 dark:text-gray-400">
            Upload a CSV list of public authorities for request drafting
          </p>
        </div>
        <Button href="/authorities" variant="outline">
          <ArrowLeft className="h-4 w-4" />
          Back
        </Button>
      </div>

      <Card>
        <CardHeader>
          <h3 className="text-base font-semibold text-gray-900 dark:text-gray-100">
            CSV upload
          </h3>
        </CardHeader>
        <CardContent>
          <form action={importAuthorities} className="grid gap-5">
            <label className="grid gap-2">
              <span className="text-sm font-medium text-gray-700 dark:text-gray-300">
                Authority CSV
              </span>
              <Input name="file" type="file" accept=".csv,text/csv" required />
            </label>

            <div className="rounded-lg border border-gray-200 bg-gray-50 p-4 text-sm text-gray-600 dark:border-gray-800 dark:bg-gray-900 dark:text-gray-400">
              Expected columns: slug, name, url. The url column is optional.
            </div>

            <div className="flex justify-end gap-3 border-t border-gray-200 pt-5 dark:border-gray-800">
              <Button href="/authorities" variant="ghost">
                Cancel
              </Button>
              <Button type="submit">
                <Upload className="h-4 w-4" />
                Import CSV
              </Button>
            </div>
          </form>
        </CardContent>
      </Card>
    </div>
  );
}
