import { EventEmitter } from "node:events";
import { PassThrough, Writable } from "node:stream";
import { describe, expect, it } from "vitest";
import { FyiMcpClient, FyiMcpError, type JsonRpcRequest } from "./mcp-client";

class MockStdin extends Writable {
  constructor(private readonly onRequest: (request: JsonRpcRequest) => void) {
    super();
  }

  _write(chunk: Buffer, _encoding: BufferEncoding, callback: (error?: Error | null) => void) {
    for (const line of chunk.toString("utf8").trim().split("\n")) {
      if (line) {
        this.onRequest(JSON.parse(line) as JsonRpcRequest);
      }
    }
    callback();
  }
}

function createMockProcess(handler: (request: JsonRpcRequest) => unknown) {
  const child = new EventEmitter() as EventEmitter & {
    stdin: MockStdin;
    stdout: PassThrough;
    stderr: PassThrough;
    kill: () => boolean;
  };

  child.stdout = new PassThrough();
  child.stderr = new PassThrough();
  child.stdin = new MockStdin((request) => {
    const result = handler(request);
    child.stdout.write(
      `${JSON.stringify({ jsonrpc: "2.0", id: request.id, result })}\n`
    );
  });
  child.kill = () => {
    child.emit("exit", 0, null);
    return true;
  };

  return child;
}

function createClient(handler: (request: JsonRpcRequest) => unknown) {
  return new FyiMcpClient({
    command: "mock-fyi-mcp",
    timeoutMs: 250,
    spawnProcess: () => createMockProcess(handler),
  });
}

describe("FyiMcpClient", () => {
  it("sends initialize as JSON-RPC 2.0", async () => {
    const client = createClient((request) => {
      expect(request).toMatchObject({
        jsonrpc: "2.0",
        id: 1,
        method: "initialize",
      });
      return { serverInfo: { name: "fyi-mcp" } };
    });

    await expect(client.initialize()).resolves.toEqual({
      serverInfo: { name: "fyi-mcp" },
    });
    await client.close();
  });

  it("calls tools/call and parses text JSON payloads", async () => {
    const client = createClient((request) => {
      expect(request.method).toBe("tools/call");
      expect(request.params).toEqual({
        name: "check_status",
        arguments: {},
      });

      return {
        content: [
          {
            type: "text",
            text: JSON.stringify({
              status: "healthy",
              database: "connected",
              metrics: { total_requests: 2, total_correspondence: 3 },
            }),
          },
        ],
      };
    });

    await expect(client.checkStatus()).resolves.toEqual({
      status: "healthy",
      database: "connected",
      metrics: { total_requests: 2, total_correspondence: 3 },
    });
    await client.close();
  });

  it("turns MCP tool errors into exceptions", async () => {
    const client = createClient(() => ({
      isError: true,
      content: [{ type: "text", text: "Request with ID 99 not found" }],
    }));

    await expect(client.retrieveRequest(99)).rejects.toThrow(
      new FyiMcpError("Request with ID 99 not found")
    );
    await client.close();
  });

  it("lists requests through the MCP list_requests tool", async () => {
    const client = createClient((request) => {
      expect(request.params).toEqual({
        name: "list_requests",
        arguments: { limit: 25 },
      });

      return {
        content: [
          {
            type: "text",
            text: JSON.stringify([
              {
                id: 1,
                title: "Rates request",
                body: "Body",
                status: "submitted",
              },
            ]),
          },
        ],
      };
    });

    await expect(client.listRequests(25)).resolves.toEqual([
      {
        id: 1,
        title: "Rates request",
        body: "Body",
        status: "submitted",
      },
    ]);
    await client.close();
  });

  it("deletes requests through the MCP delete_request tool", async () => {
    const client = createClient((request) => {
      expect(request.params).toEqual({
        name: "delete_request",
        arguments: { id: 42 },
      });

      return {
        content: [
          {
            type: "text",
            text: JSON.stringify({ deleted: true, id: 42 }),
          },
        ],
      };
    });

    await expect(client.deleteRequest(42)).resolves.toEqual({
      deleted: true,
      id: 42,
    });
    await client.close();
  });
});
