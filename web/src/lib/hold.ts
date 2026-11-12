// Scrolls an element back to where the reader left it, and keeps it there while
// what it shows settles: rows are measured, a text and its pictures arrive and push
// what is below them down. It lets go once nothing has moved for a moment, and at
// once when the reader does anything, so it never scrolls against them.

/** Where to be: `top` in pixels, `ready` once what decides the place is there. */
export interface Aim {
	top: number;
	ready: boolean;
}

/** Nothing moved for this long: the place holds. */
export const SETTLED_AFTER = 1500;
/** The longest a place is held, whatever arrives. */
export const HOLD_AT_MOST = 15_000;

/** What the reader does, which ends every hold. */
const INPUT = ['wheel', 'touchstart', 'pointerdown', 'keydown'] as const;

const holds = new Set<Hold>();

function interrupted() {
	for (const hold of [...holds]) hold.stop();
}

export class Hold {
	private frame = 0;
	private started: number;
	private changed: number;
	private lastTop = NaN;
	private lastHeight = NaN;
	private done = false;
	private scroller: HTMLElement;
	private aim: () => Aim | null;
	private onend: (() => void) | undefined;

	/**
	 * Holds `scroller` where `aim` says, frame after frame; an aim of null is not
	 * known yet. `onend` is called once when the hold lets go.
	 */
	constructor(scroller: HTMLElement, aim: () => Aim | null, onend?: () => void) {
		this.scroller = scroller;
		this.aim = aim;
		this.onend = onend;
		this.started = this.changed = performance.now();
		if (holds.size === 0 && typeof window !== 'undefined') {
			for (const kind of INPUT)
				window.addEventListener(kind, interrupted, { capture: true, passive: true });
		}
		holds.add(this);
		this.tick();
	}

	get active() {
		return !this.done;
	}

	private tick = () => {
		this.frame = 0;
		if (this.done) return;
		const time = performance.now();
		const aim = this.aim();
		const height = this.scroller.scrollHeight;
		if (aim) {
			if (aim.top !== this.lastTop || height !== this.lastHeight) this.changed = time;
			this.lastTop = aim.top;
			this.lastHeight = height;
			if (Math.abs(this.scroller.scrollTop - aim.top) >= 1) this.scroller.scrollTop = aim.top;
		}
		const settled = aim?.ready && time - this.changed >= SETTLED_AFTER;
		if (settled || time - this.started >= HOLD_AT_MOST) {
			this.stop();
			return;
		}
		this.frame = requestAnimationFrame(this.tick);
	};

	stop() {
		if (this.done) return;
		this.done = true;
		cancelAnimationFrame(this.frame);
		holds.delete(this);
		if (holds.size === 0 && typeof window !== 'undefined') {
			for (const kind of INPUT) window.removeEventListener(kind, interrupted, true);
		}
		this.onend?.();
	}
}
