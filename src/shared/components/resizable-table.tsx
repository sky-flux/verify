import { useRef } from "react";
import {
	Table,
	TableBody,
	TableCell,
	TableHead,
	TableHeader,
	TableRow,
} from "@/shared/components/ui/table";
import {
	KEYBOARD_RESIZE_STEP,
	useResizableColumns,
} from "@/shared/hooks/use-resizable-columns";
import { cn } from "@/shared/lib/utils";

export interface ResizableColumnDef<T> {
	key: string;
	label: React.ReactNode;
	/** Starting share of the table's width, as a percentage. All columns'
	 *  defaultWidthPercent values should sum to 100. */
	defaultWidthPercent: number;
	render: (row: T) => React.ReactNode;
	cellClassName?: string;
	/** Set for action cells nested inside an otherwise row-clickable table. */
	stopClickPropagation?: boolean;
}

function ResizableTableHead({
	widthPercent,
	showHandle,
	onResizeStart,
	onResizeBy,
	children,
}: {
	widthPercent: number;
	showHandle: boolean;
	onResizeStart: (e: React.MouseEvent) => void;
	onResizeBy: (delta: number) => void;
	children: React.ReactNode;
}) {
	return (
		<TableHead
			className="group/th relative overflow-hidden truncate"
			style={{ width: `${widthPercent}%` }}
		>
			{children}
			{showHandle && (
				<button
					type="button"
					aria-label="拖动调整列宽"
					onMouseDown={onResizeStart}
					onKeyDown={(e) => {
						if (e.key === "ArrowLeft") onResizeBy(-KEYBOARD_RESIZE_STEP);
						else if (e.key === "ArrowRight") onResizeBy(KEYBOARD_RESIZE_STEP);
					}}
					className={cn(
						"absolute top-0 right-0 h-full w-1.5 cursor-col-resize touch-none select-none border-0 bg-transparent p-0",
						"opacity-0 group-hover/th:opacity-100 hover:bg-primary/60 active:bg-primary",
						"focus-visible:opacity-100 focus-visible:outline-2 focus-visible:outline-ring",
					)}
				/>
			)}
		</TableHead>
	);
}

/**
 * A plain HTML table (not TanStack Table) with drag-resizable column
 * headers, shared across History/Batch/Dashboard so all three present
 * verification results with identical header behavior. Columns are sized
 * by percentage rather than fixed pixels, so they always sum to 100% of
 * the container — the table fills its width on any screen size with no
 * dead space, and dragging a handle shifts share between the two adjacent
 * columns instead of setting one column's absolute width.
 */
export function ResizableTable<T>({
	columns,
	rows,
	getRowKey,
	onRowClick,
}: {
	columns: ResizableColumnDef<T>[];
	rows: T[];
	getRowKey: (row: T) => string;
	onRowClick?: (row: T) => void;
}) {
	const containerRef = useRef<HTMLDivElement>(null);
	const { percents, startResize, resizeBy } = useResizableColumns(
		columns.map((c) => c.defaultWidthPercent),
		containerRef,
	);

	return (
		<div
			ref={containerRef}
			className="w-full overflow-x-auto rounded-lg border"
		>
			<Table className="w-full table-fixed">
				<colgroup>
					{columns.map((c, i) => (
						<col key={c.key} style={{ width: `${percents[i]}%` }} />
					))}
				</colgroup>
				<TableHeader>
					<TableRow>
						{columns.map((c, i) => (
							<ResizableTableHead
								key={c.key}
								widthPercent={percents[i]}
								showHandle={i < columns.length - 1}
								onResizeStart={startResize(i)}
								onResizeBy={(d) => resizeBy(i, d)}
							>
								{c.label}
							</ResizableTableHead>
						))}
					</TableRow>
				</TableHeader>
				<TableBody>
					{rows.map((row) => (
						<TableRow
							key={getRowKey(row)}
							className={cn(onRowClick && "cursor-pointer")}
							onClick={() => onRowClick?.(row)}
						>
							{columns.map((c) => (
								<TableCell
									key={c.key}
									className={cn("truncate", c.cellClassName)}
									onClick={
										c.stopClickPropagation
											? (e) => e.stopPropagation()
											: undefined
									}
								>
									{c.render(row)}
								</TableCell>
							))}
						</TableRow>
					))}
				</TableBody>
			</Table>
		</div>
	);
}
