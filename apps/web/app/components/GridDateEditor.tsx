'use client';

import { useState, useCallback, useMemo, useEffect } from 'react';
import { format as formatDateFns, parseISO } from 'date-fns';
import { formatDate } from '@twocents/shared';
import { ChevronLeft, ChevronRight } from 'lucide-react';
import Button from './ui/Button';
import { GridCellKind } from '@glideapps/glide-data-grid';

function commitTextCell(
  value: any,
  nextData: string,
  nextDisplayData: string | undefined,
  onFinishedEditing: (newValue?: any, movement?: readonly [0 | 1 | -1, 0 | 1 | -1]) => void
) {
  onFinishedEditing(
    {
      ...value,
      kind: GridCellKind.Text,
      data: nextData,
      displayData: nextDisplayData ?? nextData,
      allowOverlay: true,
    },
    [0, 0]
  );
}

export default function GridDateEditor(p: any) {
  const { value, onFinishedEditing } = p;
  const raw = typeof value?.data === 'string' ? value.data : '';
  const initial = raw.includes('T') ? raw.split('T')[0] : raw;

  const baseDate = useMemo(() => {
    if (!initial) return new Date();
    try {
      const d = parseISO(initial);
      return Number.isNaN(d.getTime()) ? new Date() : d;
    } catch {
      return new Date();
    }
  }, [initial]);

  const selectedY = baseDate.getFullYear();
  const selectedM = baseDate.getMonth();
  const selectedD = baseDate.getDate();

  const [viewY, setViewY] = useState<number>(selectedY);
  const [viewM, setViewM] = useState<number>(selectedM);

  useEffect(() => {
    setViewY(selectedY);
    setViewM(selectedM);
  }, [selectedY, selectedM]);

  const commitDate = useCallback(
    (d: Date) => {
      const iso = formatDateFns(d, 'yyyy-MM-dd');
      commitTextCell(value, iso, formatDate(iso), onFinishedEditing);
    },
    [onFinishedEditing, value]
  );

  const daysInMonth = useMemo(() => new Date(viewY, viewM + 1, 0).getDate(), [viewM, viewY]);
  const firstWeekday = useMemo(() => new Date(viewY, viewM, 1).getDay(), [viewM, viewY]);
  const prevMonthDaysInMonth = useMemo(() => new Date(viewY, viewM, 0).getDate(), [viewM, viewY]);

  const monthNames = useMemo(
    () => ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'],
    []
  );

  type SliderPanel = 'month' | 'year' | null;
  const [panel, setPanel] = useState<SliderPanel>(null);

  const commitDay = useCallback(
    (dayOfMonth: number) => {
      commitDate(new Date(viewY, viewM, dayOfMonth));
    },
    [commitDate, viewM, viewY]
  );

  const selectedLabel = useMemo(() => formatDateFns(baseDate, 'MMM d, yyyy'), [baseDate]);

  return (
    <div className="min-w-[240px] max-w-[270px] select-none">
      <div className="flex items-center justify-between">
        <div className="text-xs font-medium tabular-nums">
          <span
            className="rounded-notion px-1 py-0.5 hover:bg-hover cursor-pointer"
            onClick={() => setPanel((p) => (p === 'month' ? null : 'month'))}
            title="Change month"
          >
            {monthNames[viewM]}
          </span>{' '}
          <span
            className="rounded-notion px-1 py-0.5 hover:bg-hover cursor-pointer"
            onClick={() => setPanel((p) => (p === 'year' ? null : 'year'))}
            title="Change year"
          >
            {viewY}
          </span>
        </div>
        <div className="flex items-center gap-1">
          <button
            type="button"
            className="inline-flex h-6 w-6 items-center justify-center rounded-notion hover:bg-hover"
            onClick={() => {
              const prev = viewM - 1;
              if (prev < 0) {
                setViewM(11);
                setViewY((y) => y - 1);
              } else {
                setViewM(prev);
              }
            }}
            aria-label="Previous month"
          >
            <ChevronLeft className="h-3.5 w-3.5" />
          </button>
          <button
            type="button"
            className="inline-flex h-6 w-6 items-center justify-center rounded-notion hover:bg-hover"
            onClick={() => {
              const next = viewM + 1;
              if (next > 11) {
                setViewM(0);
                setViewY((y) => y + 1);
              } else {
                setViewM(next);
              }
            }}
            aria-label="Next month"
          >
            <ChevronRight className="h-3.5 w-3.5" />
          </button>
        </div>
      </div>

      {panel === 'month' ? (
        <div className="mt-2 px-0.5">
          <div className="flex items-center justify-between text-[10px] text-muted-foreground">
            <span>Month</span>
            <span className="tabular-nums">{monthNames[viewM]}</span>
          </div>
          <input
            type="range"
            min={0}
            max={11}
            step={1}
            value={viewM}
            onChange={(e) => setViewM(Number(e.target.value))}
            className={[
              'mt-1 w-full bg-transparent appearance-none focus:outline-none',
              '[&::-webkit-slider-runnable-track]:h-1',
              '[&::-webkit-slider-runnable-track]:rounded-full',
              '[&::-webkit-slider-runnable-track]:bg-border/60',
              '[&::-webkit-slider-thumb]:appearance-none',
              '[&::-webkit-slider-thumb]:h-3',
              '[&::-webkit-slider-thumb]:w-3',
              '[&::-webkit-slider-thumb]:rounded-full',
              '[&::-webkit-slider-thumb]:bg-accent',
              '[&::-webkit-slider-thumb]:border',
              '[&::-webkit-slider-thumb]:border-border',
              '[&::-webkit-slider-thumb]:mt-[-5px]',
              '[&::-moz-range-track]:h-1',
              '[&::-moz-range-track]:rounded-full',
              '[&::-moz-range-track]:bg-border/60',
              '[&::-moz-range-thumb]:h-3',
              '[&::-moz-range-thumb]:w-3',
              '[&::-moz-range-thumb]:rounded-full',
              '[&::-moz-range-thumb]:bg-accent',
              '[&::-moz-range-thumb]:border',
              '[&::-moz-range-thumb]:border-border',
            ].join(' ')}
          />
        </div>
      ) : null}

      {panel === 'year' ? (
        <div className="mt-2 px-0.5">
          <div className="flex items-center justify-between text-[10px] text-muted-foreground">
            <span>Year</span>
            <span className="tabular-nums">{viewY}</span>
          </div>
          <input
            type="range"
            min={1900}
            max={2100}
            step={1}
            value={viewY}
            onChange={(e) => setViewY(Number(e.target.value))}
            className={[
              'mt-1 w-full bg-transparent appearance-none focus:outline-none',
              '[&::-webkit-slider-runnable-track]:h-1',
              '[&::-webkit-slider-runnable-track]:rounded-full',
              '[&::-webkit-slider-runnable-track]:bg-border/60',
              '[&::-webkit-slider-thumb]:appearance-none',
              '[&::-webkit-slider-thumb]:h-3',
              '[&::-webkit-slider-thumb]:w-3',
              '[&::-webkit-slider-thumb]:rounded-full',
              '[&::-webkit-slider-thumb]:bg-accent',
              '[&::-webkit-slider-thumb]:border',
              '[&::-webkit-slider-thumb]:border-border',
              '[&::-webkit-slider-thumb]:mt-[-5px]',
              '[&::-moz-range-track]:h-1',
              '[&::-moz-range-track]:rounded-full',
              '[&::-moz-range-track]:bg-border/60',
              '[&::-moz-range-thumb]:h-3',
              '[&::-moz-range-thumb]:w-3',
              '[&::-moz-range-thumb]:rounded-full',
              '[&::-moz-range-thumb]:bg-accent',
              '[&::-moz-range-thumb]:border',
              '[&::-moz-range-thumb]:border-border',
            ].join(' ')}
          />
        </div>
      ) : null}

      <div className="mt-1 grid grid-cols-7 gap-0.5 text-[10px] text-muted-foreground">
        {['S', 'M', 'T', 'W', 'T', 'F', 'S'].map((d, idx) => (
          <div key={`${d}-${idx}`} className="h-4 flex items-center justify-center">
            {d}
          </div>
        ))}
      </div>

      <div className="grid grid-cols-7 gap-0.5">
        {Array.from({ length: 42 }, (_, i) => {
          const dayOfMonth = i - firstWeekday + 1;
          const inMonth = dayOfMonth >= 1 && dayOfMonth <= daysInMonth;
          const displayDay = inMonth
            ? dayOfMonth
            : dayOfMonth < 1
              ? prevMonthDaysInMonth + dayOfMonth
              : dayOfMonth - daysInMonth;
          const isSelected = inMonth && selectedY === viewY && selectedM === viewM && selectedD === dayOfMonth;
          return (
            <button
              key={i}
              type="button"
              className={[
                'h-7 w-7 rounded-notion text-xs flex items-center justify-center transition-colors tabular-nums',
                inMonth ? 'text-foreground' : 'text-muted-foreground',
                isSelected ? 'bg-accent text-accent-foreground' : 'hover:bg-hover',
              ].join(' ')}
              onClick={() => commitDay(dayOfMonth)}
            >
              {displayDay}
            </button>
          );
        })}
      </div>

      <div className="mt-2 flex items-center justify-between gap-2">
        <div className="text-[11px] text-muted-foreground tabular-nums">
          {selectedLabel}
        </div>
        <Button
          variant="ghost"
          size="sm"
          className="h-7 px-2 text-xs"
          type="button"
          onClick={() => {
            const t = new Date();
            setViewY(t.getFullYear());
            setViewM(t.getMonth());
            commitDate(t);
          }}
        >
          Today
        </Button>
      </div>
    </div>
  );
}

