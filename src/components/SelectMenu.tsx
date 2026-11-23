import { useCallback, useEffect, useLayoutEffect, useRef, useState, type CSSProperties, type KeyboardEvent } from "react";
import { createPortal } from "react-dom";
import { Check, ChevronDown } from "lucide-react";

export interface SelectOption<Value extends string> {
  value: Value;
  label: string;
  lang?: string;
}
interface SelectMenuProps<Value extends string> {
  id: string;
  labelId: string;
  value: Value;
  options: readonly SelectOption<Value>[];
  disabled?: boolean;
  onChange: (value: Value) => void;
}

/** A select-only listbox. Values remain owned by the surrounding settings draft. */
export function SelectMenu<Value extends string>({ id, labelId, value, options, disabled = false, onChange }: SelectMenuProps<Value>) {
  const trigger = useRef<HTMLButtonElement>(null);
  const listbox = useRef<HTMLDivElement>(null);
  const [open, setOpen] = useState(false);
  const [host, setHost] = useState<HTMLElement | null>(null);
  const [activeIndex, setActiveIndex] = useState(0);
  const [placement, setPlacement] = useState<CSSProperties | null>(null);
  const search = useRef({ text: "", at: 0 });
  const needsFocus = useRef(false);
  const selectedIndex = Math.max(0, options.findIndex(option => option.value === value));
  const selected = options[selectedIndex];
  const listboxId = id + "-listbox";

  const close = useCallback((restoreFocus = true) => {
    setOpen(false);
    needsFocus.current = false;
    search.current = { text: "", at: 0 };
    if (restoreFocus) trigger.current?.focus({ preventScroll: true });
  }, []);

  function show(index = selectedIndex) {
    if (disabled || options.length === 0) return;
    setHost(trigger.current?.closest("dialog") ?? document.body);
    setActiveIndex(index);
    needsFocus.current = true;
    setPlacement(null);
    setOpen(true);
  }

  function choose(index: number) {
    const option = options[index];
    if (!option || disabled) return;
    onChange(option.value);
    close();
  }

  useEffect(() => {
    if (disabled && open) close();
  }, [disabled, open, close]);

  useLayoutEffect(() => {
    if (!open) return;
    const button = trigger.current;
    const menu = listbox.current;
    if (!button || !menu) return;
    let frame = 0;

    const measure = () => {
      const rect = button.getBoundingClientRect();
      const viewport = window.visualViewport;
      const viewportLeft = viewport?.offsetLeft ?? 0;
      const viewportTop = viewport?.offsetTop ?? 0;
      const viewportWidth = viewport?.width ?? window.innerWidth;
      const viewportHeight = viewport?.height ?? window.innerHeight;
      const dialog = button.closest("dialog");
      const dialogRect = dialog?.getBoundingClientRect();
      const topBound = Math.max(viewportTop + 8, dialogRect ? dialogRect.top + 8 : 0);
      const bottomBound = Math.min(viewportTop + viewportHeight - 8, dialogRect ? dialogRect.bottom - 8 : Infinity);
      const leftBound = Math.max(viewportLeft + 8, dialogRect ? dialogRect.left + 8 : 0);
      const rightBound = Math.min(viewportLeft + viewportWidth - 8, dialogRect ? dialogRect.right - 8 : Infinity);
      const scroller = button.closest(".modal-body")?.getBoundingClientRect();
      if (rect.bottom <= topBound || rect.top >= bottomBound ||
          (scroller && (rect.bottom <= scroller.top || rect.top >= scroller.bottom))) {
        close();
        return;
      }
      const gap = 6;
      const below = Math.max(0, bottomBound - rect.bottom - gap);
      const above = Math.max(0, rect.top - gap - topBound);
      const desiredHeight = Math.min(menu.scrollHeight, 288);
      const down = below >= desiredHeight || below >= above;
      const maxHeight = Math.max(0, Math.min(288, down ? below : above));
      if (maxHeight < 44) { close(); return; }
      const height = Math.min(desiredHeight, maxHeight);
      const width = Math.min(Math.max(rect.width, 212), rightBound - leftBound);
      const left = Math.max(leftBound, Math.min(rect.right - width, rightBound - width));
      const top = down ? rect.bottom + gap : rect.top - gap - height;
      setPlacement({ left, top, width, maxHeight });
    };
    const schedule = () => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(measure);
    };

    const onScroll = (event: Event) => {
      // Scrolling the options does not move their anchor. Repositioning here
      // would scroll the active option back into view during a touch gesture.
      if (event.target instanceof Node && menu.contains(event.target)) return;
      schedule();
    };

    measure();
    window.addEventListener("resize", schedule);
    window.addEventListener("scroll", onScroll, true);
    window.visualViewport?.addEventListener("resize", schedule);
    window.visualViewport?.addEventListener("scroll", schedule);
    const observer = typeof ResizeObserver === "undefined" ? null : new ResizeObserver(schedule);
    observer?.observe(button);
    const dialog = button.closest("dialog");
    if (dialog) observer?.observe(dialog);

    return () => {
      cancelAnimationFrame(frame);
      window.removeEventListener("resize", schedule);
      window.removeEventListener("scroll", onScroll, true);
      window.visualViewport?.removeEventListener("resize", schedule);
      window.visualViewport?.removeEventListener("scroll", schedule);
      observer?.disconnect();
    };
  }, [open, close]);

  useLayoutEffect(() => {
    if (!open || !placement) return;
    const menu = listbox.current;
    if (!menu) return;
    // The first render is hidden until measured; focus only after that visible
    // placement is committed, and never steal focus on subsequent repositioning.
    if (needsFocus.current) {
      needsFocus.current = false;
      menu.focus({ preventScroll: true });
    }
    const option = document.getElementById(listboxId + "-option-" + activeIndex);
    if (!option) return;
    const top = option.offsetTop;
    const bottom = top + option.offsetHeight;
    if (top < menu.scrollTop) menu.scrollTop = Math.max(0, top - 6);
    else if (bottom > menu.scrollTop + menu.clientHeight) menu.scrollTop = bottom - menu.clientHeight + 6;
  }, [activeIndex, open, placement, listboxId]);

  useEffect(() => {
    if (!open) return;
    const contains = (target: EventTarget | null) => target instanceof Node &&
      (trigger.current?.contains(target) || listbox.current?.contains(target));
    const onPointerDown = (event: PointerEvent) => { if (!contains(event.target)) close(); };
    const onFocusIn = (event: FocusEvent) => { if (!contains(event.target)) close(false); };
    document.addEventListener("pointerdown", onPointerDown, true);
    document.addEventListener("focusin", onFocusIn);
    return () => {
      document.removeEventListener("pointerdown", onPointerDown, true);
      document.removeEventListener("focusin", onFocusIn);
    };
  }, [open, close]);

  function onTriggerKeyDown(event: KeyboardEvent<HTMLButtonElement>) {
    if (event.isPropagationStopped() || event.nativeEvent.isComposing || event.metaKey || event.ctrlKey || event.altKey) return;
    if (["ArrowDown", "ArrowUp", "Home", "End", "Enter", " "].includes(event.key)) {
      event.preventDefault();
      event.stopPropagation();
      show(event.key === "Home" ? 0 : event.key === "End" ? options.length - 1 : selectedIndex);
    }
  }

  function onListKeyDown(event: KeyboardEvent<HTMLDivElement>) {
    if (event.nativeEvent.isComposing || event.metaKey || event.ctrlKey || event.altKey) return;
    if (event.key === "Tab") {
      close();
      return; // Native tabbing continues from the trigger into the next settings field.
    }
    if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      close();
      return;
    }
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      event.stopPropagation();
      choose(activeIndex);
      return;
    }
    const next = event.key === "ArrowDown" ? Math.min(options.length - 1, activeIndex + 1)
      : event.key === "ArrowUp" ? Math.max(0, activeIndex - 1)
      : event.key === "Home" ? 0 : event.key === "End" ? options.length - 1 : null;
    if (next !== null) {
      event.preventDefault();
      event.stopPropagation();
      setActiveIndex(next);
      return;
    }
    if (event.key.length === 1 && !event.repeat) {
      const now = Date.now();
      const text = (now - search.current.at < 700 ? search.current.text : "") + event.key.toLocaleLowerCase();
      search.current = { text, at: now };
      const index = options.findIndex(option => option.label.toLocaleLowerCase().startsWith(text));
      if (index >= 0) {
        event.preventDefault();
        event.stopPropagation();
        setActiveIndex(index);
      }
    }
  }

  return <div className="select-control">
    <button ref={trigger} type="button" id={id} className="select-trigger" data-testid="language-select"
      disabled={disabled || options.length === 0} aria-haspopup="listbox" aria-expanded={open}
      aria-controls={open ? listboxId : undefined} aria-labelledby={labelId + " " + id + "-value"}
      onClick={() => open ? close() : show()} onKeyDown={onTriggerKeyDown}>
      <span id={id + "-value"} lang={selected?.lang}>{selected?.label}</span>
      <ChevronDown size={16} aria-hidden="true" />
    </button>
    {open && host && createPortal(<div ref={listbox} id={listboxId} className="select-popup" role="listbox" tabIndex={-1}
      data-testid="language-options" aria-labelledby={labelId} aria-activedescendant={listboxId + "-option-" + activeIndex}
      style={placement ?? { visibility: "hidden", left: 0, top: 0, width: 212 }} onKeyDown={onListKeyDown}>
      {options.map((option, index) => <div key={option.value} id={listboxId + "-option-" + index}
        role="option" className="select-option" aria-selected={option.value === value} data-active={index === activeIndex}
        data-value={option.value} lang={option.lang}
        onPointerMove={event => { if (event.pointerType === "mouse") setActiveIndex(index); }}
        onClick={() => choose(index)}>
        <span>{option.label}</span><Check size={16} aria-hidden="true" className={option.value === value ? "" : "select-check-placeholder"} />
      </div>)}
    </div>, host)}
  </div>;
}
