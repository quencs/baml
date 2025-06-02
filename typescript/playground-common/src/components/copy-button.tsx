'use client';

import { type VariantProps, cva } from 'class-variance-authority';
import type React from 'react';
import { useState } from 'react';
import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';
import { Check, Copy, Loader2 } from 'lucide-react';

export type CopyState = 'idle' | 'copying' | 'copied';

const copyButtonVariants = cva('relative', {
  defaultVariants: {
    size: 'default',
    variant: 'ghost',
  },
  variants: {
    size: {
      sm: 'h-7 w-7',
      default: 'h-9 w-9',
      lg: 'h-10 w-10',
    },
    variant: {
      ghost: 'hover:bg-accent hover:text-accent-foreground',
      outline:
        'border border-input bg-transparent hover:bg-accent hover:text-accent-foreground',
      secondary: 'bg-secondary text-secondary-foreground hover:bg-secondary/80',
      default: 'bg-primary text-primary-foreground hover:bg-primary/90',
    },
  },
});

export interface CopyButtonProps
  extends React.ButtonHTMLAttributes<HTMLButtonElement>,
    VariantProps<typeof copyButtonVariants> {
  text: string;
  onCopied?: () => void;
  showToast?: boolean;
  successMessage?: string;
  errorMessage?: string;
}

export function CopyButton({
  className,
  text,
  onCopied,
  size,
  variant,
  successMessage = 'Copied to clipboard',
  errorMessage = 'Failed to copy to clipboard',
  ...props
}: CopyButtonProps) {
  const [copyState, setCopyState] = useState<CopyState>('idle');

  const copyToClipboard = async () => {
    setCopyState('copying');
    try {
      await navigator.clipboard.writeText(text);
      setCopyState('copied');

      onCopied?.();
      setTimeout(() => setCopyState('idle'), 2000);
    } catch (err) {
      console.error('Failed to copy:', err);
      setCopyState('idle');
    }
  };

  return (
    <Button
      type="button"
      // className={cn(copyButtonVariants({ size, variant }), className)}
      className='size-7'
      variant="ghost"
      size="icon"
      onClick={copyToClipboard}
      aria-label={copyState === 'copied' ? 'Copied' : 'Copy to clipboard'}
      {...props}
    >
      {copyState === 'copying' ? (
        <Loader2
          className="animate-spin text-muted-foreground w-4 h-4"
        />
      ) : copyState === 'copied' ? (
        <Check
          className="text-primary w-4 h-4"
        />
      ) : (
        <Copy
          className="text-muted-foreground w-4 h-4"
        />
      )}
    </Button>
  );
}
