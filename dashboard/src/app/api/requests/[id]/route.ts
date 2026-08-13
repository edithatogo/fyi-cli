import { NextResponse } from "next/server";
import { FyiMcpClient, type UpdateRequestInput } from "@/lib/mcp-client";

export const dynamic = "force-dynamic";

export async function PATCH(
  request: Request,
  { params }: { params: Promise<{ id: string }> }
) {
  const { id: requestId } = await params;
  const id = Number(requestId);
  if (!Number.isFinite(id)) {
    return NextResponse.json({ error: "Invalid request ID" }, { status: 400 });
  }

  const body = (await request.json()) as Partial<UpdateRequestInput>;
  if (!body.title?.trim() || !body.body?.trim()) {
    return NextResponse.json(
      { error: "Title and body are required" },
      { status: 400 }
    );
  }

  const client = new FyiMcpClient();
  try {
    const updated = await client.updateRequest({
      id,
      title: body.title.trim(),
      body: body.body.trim(),
      user_name: body.user_name?.trim() || undefined,
      status: body.status?.trim() || undefined,
      url: body.url?.trim() || undefined,
      tags: body.tags ?? [],
    });
    return NextResponse.json(updated);
  } catch (error) {
    return NextResponse.json(
      {
        error:
          error instanceof Error ? error.message : "Unable to update request",
      },
      { status: 503 }
    );
  } finally {
    await client.close();
  }
}
