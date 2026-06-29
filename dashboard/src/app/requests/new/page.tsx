import { redirect } from "next/navigation";
import { ArrowLeft, Send } from "lucide-react";
import {
  Button,
  Card,
  CardContent,
  CardHeader,
  Input,
  Select,
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

async function createRequest(formData: FormData) {
  "use server";

  const title = formData.get("title")?.toString().trim();
  const body = formData.get("body")?.toString().trim();
  const userName = formData.get("user_name")?.toString().trim();
  const authoritySlug = formData.get("authority_slug")?.toString().trim();
  const tags = formData
    .get("tags")
    ?.toString()
    .split(",")
    .map((tag) => tag.trim())
    .filter(Boolean) ?? [];

  if (!title || !body) {
    throw new Error("Title and body are required.");
  }

  const client = new FyiMcpClient();
  try {
    await client.createRequest({
      title,
      body,
      user_name: userName || undefined,
      status: "draft",
      tags: authoritySlug ? [`authority:${authoritySlug}`, ...tags] : tags,
    });
  } finally {
    await client.close();
  }

  redirect("/requests");
}

export default async function NewRequestPage() {
  const { authorities, error } = await getAuthorities();
  const authorityOptions = authorities.map((authority) => ({
    value: authority.slug,
    label: authority.name,
  }));

  return (
    <div className="space-y-6">
      <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <h2 className="text-2xl font-bold text-gray-900 dark:text-gray-100">
            New request
          </h2>
          <p className="mt-1 text-sm text-gray-500 dark:text-gray-400">
            Draft an OIA request for tracking in the local FYI database
          </p>
        </div>
        <Button href="/requests" variant="outline">
          <ArrowLeft className="h-4 w-4" />
          Back
        </Button>
      </div>

      {error && (
        <Card>
          <CardContent className="p-6">
            <p className="text-sm font-medium text-amber-700 dark:text-amber-300">
              Authorities could not be loaded.
            </p>
            <p className="mt-1 text-sm text-gray-500 dark:text-gray-400">
              {error}
            </p>
          </CardContent>
        </Card>
      )}

      <Card>
        <CardHeader>
          <h3 className="text-base font-semibold text-gray-900 dark:text-gray-100">
            Request details
          </h3>
        </CardHeader>
        <CardContent>
          <form action={createRequest} className="grid gap-5">
            <label className="grid gap-2">
              <span className="text-sm font-medium text-gray-700 dark:text-gray-300">
                Authority
              </span>
              <Select
                name="authority_slug"
                options={authorityOptions}
                placeholder={
                  authorityOptions.length > 0
                    ? "Select an authority"
                    : "No authorities available"
                }
                disabled={authorityOptions.length === 0}
              />
            </label>

            <label className="grid gap-2">
              <span className="text-sm font-medium text-gray-700 dark:text-gray-300">
                Request title
              </span>
              <Input
                name="title"
                required
                placeholder="Brief public-facing request title"
                maxLength={180}
              />
            </label>

            <label className="grid gap-2">
              <span className="text-sm font-medium text-gray-700 dark:text-gray-300">
                Request body
              </span>
              <textarea
                name="body"
                required
                rows={10}
                className="w-full rounded-lg border border-gray-300 bg-white px-3 py-2 text-sm text-gray-900 placeholder:text-gray-400 focus-visible:border-brand-500 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-brand-500 disabled:cursor-not-allowed disabled:opacity-50 dark:border-gray-600 dark:bg-gray-900 dark:text-gray-100 dark:placeholder:text-gray-500"
                placeholder="Write the request text to track."
              />
            </label>

            <div className="grid gap-5 md:grid-cols-2">
              <label className="grid gap-2">
                <span className="text-sm font-medium text-gray-700 dark:text-gray-300">
                  Requester
                </span>
                <Input name="user_name" placeholder="Optional requester name" />
              </label>

              <label className="grid gap-2">
                <span className="text-sm font-medium text-gray-700 dark:text-gray-300">
                  Tags
                </span>
                <Input name="tags" placeholder="privacy, rates, policy" />
              </label>
            </div>

            <div className="flex justify-end gap-3 border-t border-gray-200 pt-5 dark:border-gray-800">
              <Button href="/requests" variant="ghost">
                Cancel
              </Button>
              <Button type="submit">
                <Send className="h-4 w-4" />
                Create draft
              </Button>
            </div>
          </form>
        </CardContent>
      </Card>
    </div>
  );
}
