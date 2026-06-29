"use client";

import { useEffect, useMemo, useRef, useState } from "react";
import { Badge, Card, CardContent, CardHeader, Input, Select } from "@/components/ui";
import type { FyiRequest } from "@/lib/mcp-client";

interface RequestInlineEditorProps {
  request: FyiRequest;
}

const statusOptions = [
  { value: "draft", label: "Draft" },
  { value: "submitted", label: "Submitted" },
  { value: "awaiting_response", label: "Awaiting response" },
  { value: "partial", label: "Partial response" },
  { value: "completed", label: "Completed" },
  { value: "closed", label: "Closed" },
];

type SaveState = "idle" | "saving" | "saved" | "error";

function splitTags(value: string) {
  return value
    .split(",")
    .map((tag) => tag.trim())
    .filter(Boolean);
}

export function RequestInlineEditor({ request }: RequestInlineEditorProps) {
  const [title, setTitle] = useState(request.title);
  const [body, setBody] = useState(request.body);
  const [status, setStatus] = useState(request.status ?? "draft");
  const [userName, setUserName] = useState(request.user_name ?? "");
  const [url, setUrl] = useState(request.url ?? "");
  const [tags, setTags] = useState(request.tags?.join(", ") ?? "");
  const [saveState, setSaveState] = useState<SaveState>("idle");
  const hasMounted = useRef(false);

  const payload = useMemo(
    () => ({
      title,
      body,
      status,
      user_name: userName || undefined,
      url: url || undefined,
      tags: splitTags(tags),
    }),
    [body, status, tags, title, url, userName]
  );

  useEffect(() => {
    if (!hasMounted.current) {
      hasMounted.current = true;
      return;
    }

    const controller = new AbortController();
    const timeout = window.setTimeout(async () => {
      setSaveState("saving");
      try {
        const response = await fetch(`/api/requests/${request.id}`, {
          method: "PATCH",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(payload),
          signal: controller.signal,
        });
        if (!response.ok) {
          throw new Error(`Auto-save failed with ${response.status}`);
        }
        setSaveState("saved");
      } catch (error) {
        if (!controller.signal.aborted) {
          setSaveState("error");
        }
      }
    }, 750);

    return () => {
      controller.abort();
      window.clearTimeout(timeout);
    };
  }, [payload, request.id]);

  return (
    <Card>
      <CardHeader>
        <div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
          <h3 className="text-base font-semibold text-gray-900 dark:text-gray-100">
            Inline edit
          </h3>
          {saveState !== "idle" && (
            <Badge variant={saveState === "error" ? "danger" : "info"}>
              {saveState === "saving"
                ? "Saving"
                : saveState === "saved"
                  ? "Saved"
                  : "Save failed"}
            </Badge>
          )}
        </div>
      </CardHeader>
      <CardContent>
        <div className="grid gap-5">
          <label className="grid gap-2">
            <span className="text-sm font-medium text-gray-700 dark:text-gray-300">
              Title
            </span>
            <Input
              aria-label="Title"
              required
              value={title}
              onChange={(event) => setTitle(event.target.value)}
            />
          </label>

          <label className="grid gap-2">
            <span className="text-sm font-medium text-gray-700 dark:text-gray-300">
              Body
            </span>
            <textarea
              aria-label="Body"
              required
              rows={10}
              value={body}
              onChange={(event) => setBody(event.target.value)}
              className="w-full rounded-lg border border-gray-300 bg-white px-3 py-2 text-sm text-gray-900 placeholder:text-gray-400 focus-visible:border-brand-500 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-brand-500 dark:border-gray-600 dark:bg-gray-900 dark:text-gray-100 dark:placeholder:text-gray-500"
            />
          </label>

          <div className="grid gap-5 md:grid-cols-2">
            <label className="grid gap-2">
              <span className="text-sm font-medium text-gray-700 dark:text-gray-300">
                Status
              </span>
              <Select
                aria-label="Status"
                options={statusOptions}
                value={status}
                onChange={(event) => setStatus(event.target.value)}
              />
            </label>

            <label className="grid gap-2">
              <span className="text-sm font-medium text-gray-700 dark:text-gray-300">
                Requester
              </span>
              <Input
                aria-label="Requester"
                value={userName}
                onChange={(event) => setUserName(event.target.value)}
              />
            </label>
          </div>

          <div className="grid gap-5 md:grid-cols-2">
            <label className="grid gap-2">
              <span className="text-sm font-medium text-gray-700 dark:text-gray-300">
                Source URL
              </span>
              <Input
                aria-label="Source URL"
                value={url}
                onChange={(event) => setUrl(event.target.value)}
              />
            </label>

            <label className="grid gap-2">
              <span className="text-sm font-medium text-gray-700 dark:text-gray-300">
                Tags
              </span>
              <Input
                aria-label="Tags"
                value={tags}
                onChange={(event) => setTags(event.target.value)}
              />
            </label>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
