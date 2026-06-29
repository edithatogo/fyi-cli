"use client";

import { useMemo, useState } from "react";
import { FileText, Search } from "lucide-react";
import {
  Badge,
  Card,
  CardContent,
  Input,
  Select,
  Table,
  Tbody,
  Td,
  Th,
  Thead,
  Tr,
} from "@/components/ui";
import type { FyiRequest } from "@/lib/mcp-client";

interface RequestsTableProps {
  requests: FyiRequest[];
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

function searchableText(request: FyiRequest) {
  return [
    request.title,
    request.body,
    request.status,
    request.authority_name,
    request.authority_slug,
    request.user_name,
    request.url,
    ...(request.tags ?? []),
  ]
    .filter(Boolean)
    .join(" ")
    .toLowerCase();
}

export function RequestsTable({ requests }: RequestsTableProps) {
  const [query, setQuery] = useState("");
  const [statusFilter, setStatusFilter] = useState("");
  const [authorityFilter, setAuthorityFilter] = useState("");
  const [updatedFrom, setUpdatedFrom] = useState("");
  const [updatedTo, setUpdatedTo] = useState("");

  const statusOptions = useMemo(
    () => [
      { value: "", label: "All statuses" },
      ...Array.from(
        new Set(requests.map((request) => request.status).filter(Boolean))
      )
        .sort()
        .map((status) => ({
          value: status ?? "",
          label: status ?? "",
        })),
    ],
    [requests]
  );

  const authorityOptions = useMemo(
    () => [
      { value: "", label: "All authorities" },
      ...Array.from(
        new Map(
          requests
            .map((request) => {
              const value = request.authority_slug ?? request.authority_name;
              const label = request.authority_name ?? request.authority_slug;
              return value && label ? [value, label] : undefined;
            })
            .filter((entry): entry is [string, string] => Boolean(entry))
        )
      )
        .sort((left, right) => left[1].localeCompare(right[1]))
        .map(([value, label]) => ({ value, label })),
    ],
    [requests]
  );

  const filteredRequests = useMemo(() => {
    const normalizedQuery = query.trim().toLowerCase();
    return requests.filter((request) => {
      const authorityValue = request.authority_slug ?? request.authority_name ?? "";
      const updatedDate = (request.updated_at ?? request.created_at ?? "").slice(0, 10);
      return (
        (!normalizedQuery || searchableText(request).includes(normalizedQuery)) &&
        (!statusFilter || request.status === statusFilter) &&
        (!authorityFilter || authorityValue === authorityFilter) &&
        (!updatedFrom || updatedDate >= updatedFrom) &&
        (!updatedTo || updatedDate <= updatedTo)
      );
    });
  }, [authorityFilter, query, requests, statusFilter, updatedFrom, updatedTo]);

  return (
    <div className="space-y-4">
      <div className="grid gap-4 lg:grid-cols-[minmax(0,1fr)_180px_220px_160px_160px]">
        <label className="grid gap-2">
          <span className="text-sm font-medium text-gray-700 dark:text-gray-300">
            Search requests
          </span>
          <div className="relative">
            <Search className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-gray-400" />
            <Input
              aria-label="Search requests"
              className="pl-9"
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              placeholder="Search title, body, requester, status, or tags"
            />
          </div>
        </label>

        <label className="grid gap-2">
          <span className="text-sm font-medium text-gray-700 dark:text-gray-300">
            Status
          </span>
          <Select
            aria-label="Status filter"
            options={statusOptions}
            value={statusFilter}
            onChange={(event) => setStatusFilter(event.target.value)}
          />
        </label>

        <label className="grid gap-2">
          <span className="text-sm font-medium text-gray-700 dark:text-gray-300">
            Authority
          </span>
          <Select
            aria-label="Authority filter"
            options={authorityOptions}
            value={authorityFilter}
            onChange={(event) => setAuthorityFilter(event.target.value)}
          />
        </label>

        <label className="grid gap-2">
          <span className="text-sm font-medium text-gray-700 dark:text-gray-300">
            Updated from
          </span>
          <Input
            aria-label="Updated from"
            type="date"
            value={updatedFrom}
            onChange={(event) => setUpdatedFrom(event.target.value)}
          />
        </label>

        <label className="grid gap-2">
          <span className="text-sm font-medium text-gray-700 dark:text-gray-300">
            Updated to
          </span>
          <Input
            aria-label="Updated to"
            type="date"
            value={updatedTo}
            onChange={(event) => setUpdatedTo(event.target.value)}
          />
        </label>
      </div>

      <Card>
        <CardContent className="p-0">
          {filteredRequests.length > 0 ? (
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
                {filteredRequests.map((request) => (
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
                {requests.length > 0
                  ? "No requests match the current search."
                  : "Requests will appear here after they are imported or created through the MCP backend."}
              </p>
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
