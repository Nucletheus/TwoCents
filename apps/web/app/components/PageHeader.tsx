'use client';

import React from 'react';

interface PageHeaderProps {
  title: string;
  subtitle?: string;
  leading?: React.ReactNode;
  householdSelect?: React.ReactNode;
  secondary?: React.ReactNode;
  secondarySlotRef?: (el: HTMLDivElement | null) => void;
}

export default function PageHeader({
  title,
  subtitle,
  leading,
  householdSelect,
  secondary,
  secondarySlotRef,
}: PageHeaderProps) {
  return (
    <div className="backdrop-blur-md bg-background/80 border-b border-border/60 shadow-sm">
      <div className="flex flex-col gap-3 px-1 py-3 sm:px-2">
        <div className="flex flex-col gap-1 sm:flex-row sm:items-center sm:justify-between">
          <div className="flex min-w-0 flex-1 items-start gap-3">
            {leading && <div className="flex-shrink-0">{leading}</div>}
            <div className="min-w-0 space-y-1">
              <h1 className="text-xl font-semibold leading-tight">{title}</h1>
              {subtitle && <p className="text-sm text-muted-foreground">{subtitle}</p>}
            </div>
          </div>
          {householdSelect && (
            <div className="flex flex-col gap-1">
              <span className="text-xs font-medium text-muted-foreground">Household</span>
              {householdSelect}
            </div>
          )}
        </div>
        {(secondary || secondarySlotRef) && (
          <div className="-mx-1 sm:-mx-2 bg-card/40 backdrop-blur-md px-1 sm:px-2 py-2">
            <div ref={secondarySlotRef} className="flex flex-wrap items-center gap-2">
              {secondary}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

