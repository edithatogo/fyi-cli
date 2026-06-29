import { spawn } from "node:child_process";
import type { EventEmitter } from "node:events";
import { createInterface, type Interface } from "node:readline";
import type { Readable, Writable } from "node:stream";

export type JsonRpcId = number | string;
export type JsonValue =
  | string
  | number
  | boolean
  | null
  | JsonValue[]
  | { [key: string]: JsonValue };

export interface JsonRpcRequest {
  jsonrpc: "2.0";
  id: JsonRpcId;
  method: string;
  params?: JsonValue;
}

export interface JsonRpcError {
  code: number;
  message: string;
  data?: JsonValue;
}

export interface JsonRpcResponse<T = JsonValue> {
  jsonrpc: "2.0";
  id?: JsonRpcId;
  result?: T;
  error?: JsonRpcError;
}

export interface McpTextContent {
  type: "text";
  text: string;
}

export interface McpToolResult {
  content?: McpTextContent[];
  isError?: boolean;
}

export interface FyiRequest {
  id: number;
  title: string;
  body: string;
  user_name?: string | null;
  authority_slug?: string | null;
  authority_name?: string | null;
  status?: string | null;
  created_at?: string | null;
  updated_at?: string | null;
  url?: string | null;
  tags?: string[] | null;
}

export interface FyiCorrespondence {
  direction: "request" | "response" | string;
  body: string;
  sent_at: string;
  state?: string | null;
  attachments?: string[] | null;
}

export interface FyiAuthority {
  slug: string;
  name: string;
  url?: string | null;
}

export interface FyiStatus {
  status: "healthy" | "unhealthy" | string;
  database: "connected" | "disconnected" | string;
  metrics: {
    total_requests: number;
    total_correspondence: number;
  };
}

export interface DeleteRequestResult {
  deleted: boolean;
  id: number;
}

export interface ImportAuthoritiesResult {
  imported: number;
}

export interface FyiRequestWithCorrespondence {
  request: FyiRequest;
  correspondence: FyiCorrespondence[];
}

export interface CreateRequestInput {
  title: string;
  body: string;
  user_name?: string;
  status?: string;
  url?: string;
  tags?: string[];
}

export interface UpdateRequestInput extends CreateRequestInput {
  id: number;
}

interface PendingRequest {
  resolve: (value: JsonRpcResponse) => void;
  reject: (reason: Error) => void;
  timeout: ReturnType<typeof setTimeout>;
}

interface McpProcess extends EventEmitter {
  stdin: Writable;
  stdout: Readable;
  kill: () => boolean;
}

interface FyiMcpClientOptions {
  command?: string;
  args?: string[];
  cwd?: string;
  env?: NodeJS.ProcessEnv;
  timeoutMs?: number;
  spawnProcess?: (command: string, args: string[], options: SpawnOptions) => McpProcess;
}

interface SpawnOptions {
  cwd?: string;
  env: NodeJS.ProcessEnv;
}

export class FyiMcpError extends Error {
  constructor(
    message: string,
    public readonly code?: number,
    public readonly data?: JsonValue
  ) {
    super(message);
    this.name = "FyiMcpError";
  }
}

export class FyiMcpClient {
  private process?: McpProcess;
  private reader?: Interface;
  private nextId = 1;
  private readonly pending = new Map<JsonRpcId, PendingRequest>();
  private readonly command: string;
  private readonly args: string[];
  private readonly timeoutMs: number;
  private readonly cwd?: string;
  private readonly env: NodeJS.ProcessEnv;
  private readonly spawnProcess: NonNullable<FyiMcpClientOptions["spawnProcess"]>;

  constructor(options: FyiMcpClientOptions = {}) {
    this.command = options.command ?? process.env.FYI_MCP_COMMAND ?? "fyi-mcp";
    this.args = options.args ?? [];
    this.cwd = options.cwd;
    this.env = { ...process.env, ...options.env };
    this.timeoutMs = options.timeoutMs ?? 10_000;
    this.spawnProcess = options.spawnProcess ?? spawn;
  }

  connect(): void {
    if (this.process) {
      return;
    }

    const child = this.spawnProcess(this.command, this.args, {
      cwd: this.cwd,
      env: this.env,
    });

    this.process = child;
    this.reader = createInterface({ input: child.stdout });

    this.reader.on("line", (line) => this.handleLine(line));
    child.once("error", (error) => this.rejectAll(error));
    child.once("exit", (code, signal) => {
      const reason = signal
        ? `fyi-mcp exited from signal ${signal}`
        : `fyi-mcp exited with code ${code ?? "unknown"}`;
      this.process = undefined;
      this.reader?.close();
      this.reader = undefined;
      this.rejectAll(new FyiMcpError(reason));
    });
  }

  async initialize(): Promise<JsonValue> {
    return this.request("initialize");
  }

  async listTools(): Promise<JsonValue> {
    return this.request("tools/list");
  }

  async callTool<T = JsonValue>(name: string, argumentsValue: JsonValue = {}): Promise<T> {
    const result = await this.request<McpToolResult>("tools/call", {
      name,
      arguments: argumentsValue,
    });

    if (result.isError) {
      throw new FyiMcpError(extractToolText(result) ?? `MCP tool '${name}' failed`);
    }

    const text = extractToolText(result);
    if (!text) {
      return result as T;
    }

    return JSON.parse(text) as T;
  }

  async retrieveRequest(id: number): Promise<FyiRequestWithCorrespondence> {
    return this.callTool<FyiRequestWithCorrespondence>("retrieve_request", { id });
  }

  async listRequests(limit = 100): Promise<FyiRequest[]> {
    return this.callTool<FyiRequest[]>("list_requests", { limit });
  }

  async createRequest(input: CreateRequestInput): Promise<FyiRequest> {
    return this.callTool<FyiRequest>("create_request", input as unknown as JsonValue);
  }

  async updateRequest(input: UpdateRequestInput): Promise<FyiRequest> {
    return this.callTool<FyiRequest>("update_request", input as unknown as JsonValue);
  }

  async deleteRequest(id: number): Promise<DeleteRequestResult> {
    return this.callTool<DeleteRequestResult>("delete_request", { id });
  }

  async listAuthorities(): Promise<FyiAuthority[]> {
    return this.callTool<FyiAuthority[]>("list_authorities");
  }

  async importAuthorities(authorities: FyiAuthority[]): Promise<ImportAuthoritiesResult> {
    return this.callTool<ImportAuthoritiesResult>("import_authorities", {
      authorities: authorities as unknown as JsonValue,
    });
  }

  async checkStatus(): Promise<FyiStatus> {
    return this.callTool<FyiStatus>("check_status");
  }

  async close(): Promise<void> {
    for (const pending of Array.from(this.pending.values())) {
      clearTimeout(pending.timeout);
      pending.reject(new FyiMcpError("MCP client closed"));
    }
    this.pending.clear();
    this.reader?.close();
    this.reader = undefined;

    if (this.process) {
      this.process.kill();
      this.process = undefined;
    }
  }

  private async request<T = JsonValue>(method: string, params?: JsonValue): Promise<T> {
    this.connect();
    const child = this.process;
    if (!child) {
      throw new FyiMcpError("fyi-mcp process is not available");
    }

    const id = this.nextId++;
    const request: JsonRpcRequest = {
      jsonrpc: "2.0",
      id,
      method,
      ...(params === undefined ? {} : { params }),
    };

    const response = await new Promise<JsonRpcResponse<T>>((resolve, reject) => {
      const timeout = setTimeout(() => {
        this.pending.delete(id);
        reject(new FyiMcpError(`MCP request '${method}' timed out after ${this.timeoutMs}ms`));
      }, this.timeoutMs);

      this.pending.set(id, {
        resolve: resolve as (value: JsonRpcResponse) => void,
        reject,
        timeout,
      });

      child.stdin.write(`${JSON.stringify(request)}\n`, (error) => {
        if (error) {
          clearTimeout(timeout);
          this.pending.delete(id);
          reject(error);
        }
      });
    });

    if (response.error) {
      throw new FyiMcpError(response.error.message, response.error.code, response.error.data);
    }

    return response.result as T;
  }

  private handleLine(line: string): void {
    if (!line.trim()) {
      return;
    }

    let response: JsonRpcResponse;
    try {
      response = JSON.parse(line) as JsonRpcResponse;
    } catch {
      this.rejectAll(new FyiMcpError(`Invalid JSON-RPC response: ${line}`));
      return;
    }

    if (response.id === undefined) {
      return;
    }

    const pending = this.pending.get(response.id);
    if (!pending) {
      return;
    }

    clearTimeout(pending.timeout);
    this.pending.delete(response.id);
    pending.resolve(response);
  }

  private rejectAll(error: Error): void {
    for (const pending of Array.from(this.pending.values())) {
      clearTimeout(pending.timeout);
      pending.reject(error);
    }
    this.pending.clear();
  }
}

function extractToolText(result: McpToolResult): string | undefined {
  return result.content?.find((item) => item.type === "text")?.text;
}
