import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { Table, Thead, Tbody, Tr, Th, Td } from "./Table";

describe("Table", () => {
  it("renders table with headers and data", () => {
    render(
      <Table>
        <Thead>
          <Tr>
            <Th>Name</Th>
            <Th>Status</Th>
          </Tr>
        </Thead>
        <Tbody>
          <Tr>
            <Td>Request 1</Td>
            <Td>Active</Td>
          </Tr>
        </Tbody>
      </Table>
    );
    expect(screen.getByText("Name")).toBeDefined();
    expect(screen.getByText("Status")).toBeDefined();
    expect(screen.getByText("Request 1")).toBeDefined();
    expect(screen.getByText("Active")).toBeDefined();
  });

  it("applies className to table", () => {
    render(
      <Table className="custom-table">
        <Thead>
          <Tr>
            <Th>Test</Th>
          </Tr>
        </Thead>
      </Table>
    );
    const table = screen.getByRole("table");
    expect(table.className).toContain("custom-table");
  });

  it("renders empty state", () => {
    render(
      <Table>
        <Thead>
          <Tr>
            <Th>Col</Th>
          </Tr>
        </Thead>
        <Tbody>
        </Tbody>
      </Table>
    );
    expect(screen.getByText("Col")).toBeDefined();
  });
});