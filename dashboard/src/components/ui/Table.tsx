import { type HTMLAttributes, type TdHTMLAttributes, type ThHTMLAttributes, type ReactNode } from "react";
import { clsx } from "clsx";

interface TableProps extends HTMLAttributes<HTMLTableElement> {
  children: ReactNode;
}

export function Table({ className, children, ...props }: TableProps) {
  return (
    <div className="w-full overflow-auto">
      <table
        className={clsx("w-full caption-bottom text-sm", className)}
        {...props}
      >
        {children}
      </table>
    </div>
  );
}

interface SectionProps extends HTMLAttributes<HTMLTableSectionElement> {
  children?: ReactNode;
}

export function Thead({ className, children, ...props }: SectionProps) {
  return (
    <thead className={clsx("[&_tr]:border-b", className)} {...props}>
      {children}
    </thead>
  );
}

export function Tbody({ className, children, ...props }: SectionProps) {
  return (
    <tbody className={clsx("[&_tr:last-child]:border-0", className)} {...props}>
      {children}
    </tbody>
  );
}

interface TrProps extends HTMLAttributes<HTMLTableRowElement> {
  children: ReactNode;
}

export function Tr({ className, children, ...props }: TrProps) {
  return (
    <tr
      className={clsx(
        "border-b border-gray-200 transition-colors hover:bg-gray-50/50 dark:border-gray-800 dark:hover:bg-gray-800/50",
        className
      )}
      {...props}
    >
      {children}
    </tr>
  );
}

interface ThProps extends ThHTMLAttributes<HTMLTableCellElement> {
  children: ReactNode;
}

export function Th({ className, children, ...props }: ThProps) {
  return (
    <th
      className={clsx(
        "h-12 px-4 text-left align-middle text-xs font-medium uppercase tracking-wider text-gray-500 dark:text-gray-400",
        className
      )}
      {...props}
    >
      {children}
    </th>
  );
}

interface TdProps extends TdHTMLAttributes<HTMLTableCellElement> {
  children: ReactNode;
}

export function Td({ className, children, ...props }: TdProps) {
  return (
    <td className={clsx("p-4 align-middle text-gray-900 dark:text-gray-100", className)} {...props}>
      {children}
    </td>
  );
}
