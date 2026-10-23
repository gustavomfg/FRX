import { forwardRef, type ButtonHTMLAttributes, type ReactNode } from 'react'
import { LoaderCircle } from 'lucide-react'
import { cn } from '@/lib/cn'

type Variant = 'primary' | 'secondary' | 'ghost' | 'danger' | 'outline'
type Size = 'sm' | 'md' | 'lg'

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: Variant
  size?: Size
  icon?: ReactNode
  iconRight?: ReactNode
  loading?: boolean
}

const variants: Record<Variant, string> = {
  primary:
    'bg-accent text-accent-ink hover:bg-accent-strong shadow-[inset_0_-2px_0_rgb(0_0_0/0.18)] active:shadow-none font-semibold',
  secondary: 'bg-raised text-fg hover:bg-overlay border border-line-2',
  outline: 'bg-transparent text-fg border border-line-2 hover:border-line-3 hover:bg-raised/60',
  ghost: 'bg-transparent text-fg-2 hover:text-fg hover:bg-raised',
  danger: 'bg-bad-soft text-bad hover:bg-bad hover:text-inverse border border-bad/25',
}

const sizes: Record<Size, string> = {
  sm: 'h-8 px-3 text-[13px] gap-1.5 rounded-sm',
  md: 'h-9 px-3.5 text-[13.5px] gap-2 rounded-md',
  lg: 'h-11 px-5 text-[14.5px] gap-2.5 rounded-md',
}

export const Button = forwardRef<HTMLButtonElement, ButtonProps>(function Button(
  { variant = 'secondary', size = 'md', icon, iconRight, loading, className, children, disabled, ...rest },
  ref,
) {
  return (
    <button
      ref={ref}
      disabled={disabled || loading}
      className={cn(
        'inline-flex shrink-0 items-center justify-center whitespace-nowrap font-medium select-none',
        'transition-[background-color,color,border-color,transform,box-shadow] duration-(--frx-dur-fast) ease-frx',
        'active:translate-y-px disabled:opacity-45 disabled:active:translate-y-0',
        variants[variant],
        sizes[size],
        className,
      )}
      {...rest}
    >
      {loading ? <LoaderCircle className="size-4 animate-spin" aria-hidden /> : icon}
      {children}
      {iconRight}
    </button>
  )
})

export function IconButton({ label, className, children, ...rest }: ButtonHTMLAttributes<HTMLButtonElement> & { label: string }) {
  return (
    <button
      aria-label={label}
      title={label}
      className={cn(
        'inline-grid size-8 shrink-0 place-items-center rounded-sm text-fg-2 transition-colors duration-(--frx-dur-fast)',
        'hover:bg-raised hover:text-fg active:translate-y-px',
        className,
      )}
      {...rest}
    >
      {children}
    </button>
  )
}
