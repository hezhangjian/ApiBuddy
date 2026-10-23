import { useEffect } from "react";
import { rpc, unwrap } from "./rpc";

function isEditable(el: EventTarget | null): HTMLElement | null {
	if (!(el instanceof HTMLElement)) return null;
	const tag = el.tagName.toLowerCase();
	if (tag === "input" || tag === "textarea") return el;
	if (el.isContentEditable) return el;
	return null;
}

function setNativeValue(el: HTMLInputElement | HTMLTextAreaElement, value: string) {
  const prototype = el instanceof HTMLInputElement ? HTMLInputElement.prototype : HTMLTextAreaElement.prototype;
  Object.getOwnPropertyDescriptor(prototype, "value")?.set?.call(el, value);
}

function insertAtCursor(el: HTMLInputElement | HTMLTextAreaElement, text: string) {
	if (el.selectionStart === null) {
		setNativeValue(el, text);
		el.dispatchEvent(new Event("input", { bubbles: true }));
		return;
	}
	const start = el.selectionStart ?? el.value.length;
	const end = el.selectionEnd ?? el.value.length;
	const before = el.value.slice(0, start);
	const after = el.value.slice(end);
	setNativeValue(el, before + text + after);
	const pos = start + text.length;
	el.selectionStart = pos;
	el.selectionEnd = pos;
	el.dispatchEvent(new Event("input", { bubbles: true }));
}

export function useClipboardShortcuts() {
	useEffect(() => {
		const onKeyDown = async (e: KeyboardEvent) => {
			if (!e.metaKey && !e.ctrlKey) return;
			const key = e.key.toLowerCase();
			if (key !== "v" && key !== "c" && key !== "x" && key !== "a") return;
			const target = isEditable(e.target);
			if (!target) {
				if (key === "a") return;
				return;
			}
			if (target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement) {
				if (target.disabled) return;
				if (target.readOnly && (key === "v" || key === "x")) { e.preventDefault(); return; }
			}

			if (key === "a") {
				if (target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement) {
					e.preventDefault();
					target.select();
				}
			} else if (key === "v") {
				e.preventDefault();
				try {
					const text = unwrap(await rpc.request["clipboard-read"](undefined));
					if (text) {
						if (
							target instanceof HTMLInputElement ||
							target instanceof HTMLTextAreaElement
						) {
							insertAtCursor(target, text);
						} else {
							document.execCommand("insertText", false, text);
						}
					}
				} catch {}
			} else if (key === "c" || key === "x") {
				if (
					target instanceof HTMLInputElement ||
					target instanceof HTMLTextAreaElement
				) {
					const start = target.selectionStart ?? 0;
					const end = target.selectionEnd ?? 0;
					if (start === end) return;
					e.preventDefault();
					const sel = target.value.slice(start, end);
					try {
						unwrap(await rpc.request["clipboard-write"]({ text: sel }));
					} catch { return; }
					if (key === "x") {
						const before = target.value.slice(0, start);
						const after = target.value.slice(end);
						setNativeValue(target, before + after);
						target.selectionStart = start;
						target.selectionEnd = start;
						target.dispatchEvent(new Event("input", { bubbles: true }));
					}
				}
			}
		};

		window.addEventListener("keydown", onKeyDown, true);
		return () => window.removeEventListener("keydown", onKeyDown, true);
	}, []);
}
