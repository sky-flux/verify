import type { RefObject } from "react";
import { useCallback, useEffect, useRef, useState } from "react";

const MIN_PERCENT = 8;
export const KEYBOARD_RESIZE_STEP = 2;

/**
 * Drives drag-to-resize (and arrow-key resize) for a row of table columns
 * sized by percentage rather than fixed pixels, so the columns always sum
 * to 100% of the container — no fixed-width dead space, no auto-filling a
 * single column unnaturally. Dragging the handle after column `index`
 * transfers share between it and the next column (like a VS Code pane
 * splitter), keeping the total fixed at 100%.
 */
export function useResizableColumns(
	defaultPercents: number[],
	containerRef: RefObject<HTMLElement | null>,
) {
	const [percents, setPercents] = useState<number[]>(defaultPercents);
	const resizing = useRef<{
		index: number;
		startX: number;
		startA: number;
		startB: number;
		containerWidth: number;
	} | null>(null);

	const applyDelta = useCallback((index: number, deltaPercent: number) => {
		setPercents((p) => {
			const next = [...p];
			let a = next[index] + deltaPercent;
			let b = next[index + 1] - deltaPercent;
			if (a < MIN_PERCENT) {
				b -= MIN_PERCENT - a;
				a = MIN_PERCENT;
			}
			if (b < MIN_PERCENT) {
				a -= MIN_PERCENT - b;
				b = MIN_PERCENT;
			}
			next[index] = a;
			next[index + 1] = b;
			return next;
		});
	}, []);

	const handleMouseMove = useCallback((e: MouseEvent) => {
		const r = resizing.current;
		if (!r) return;
		const deltaPercent = ((e.clientX - r.startX) / r.containerWidth) * 100;
		setPercents((p) => {
			const next = [...p];
			let a = r.startA + deltaPercent;
			let b = r.startB - deltaPercent;
			if (a < MIN_PERCENT) {
				b -= MIN_PERCENT - a;
				a = MIN_PERCENT;
			}
			if (b < MIN_PERCENT) {
				a -= MIN_PERCENT - b;
				b = MIN_PERCENT;
			}
			next[r.index] = a;
			next[r.index + 1] = b;
			return next;
		});
	}, []);

	const handleMouseUp = useCallback(() => {
		resizing.current = null;
		window.removeEventListener("mousemove", handleMouseMove);
		window.removeEventListener("mouseup", handleMouseUp);
	}, [handleMouseMove]);

	// Cleanup only guards against an unmount mid-drag; the listeners are
	// otherwise added/removed in matching pairs around each drag gesture.
	useEffect(() => {
		return () => {
			window.removeEventListener("mousemove", handleMouseMove);
			window.removeEventListener("mouseup", handleMouseUp);
		};
	}, [handleMouseMove, handleMouseUp]);

	const startResize = useCallback(
		(index: number) => (e: React.MouseEvent) => {
			e.preventDefault();
			resizing.current = {
				index,
				startX: e.clientX,
				startA: percents[index],
				startB: percents[index + 1],
				containerWidth: containerRef.current?.offsetWidth || 1,
			};
			window.addEventListener("mousemove", handleMouseMove);
			window.addEventListener("mouseup", handleMouseUp);
		},
		[percents, handleMouseMove, handleMouseUp, containerRef],
	);

	// Keyboard equivalent for the drag handle (arrow keys).
	const resizeBy = useCallback(
		(index: number, deltaPercent: number) => applyDelta(index, deltaPercent),
		[applyDelta],
	);

	return { percents, startResize, resizeBy };
}
