import { useState } from 'react'
import type { FixPack } from '@/types'
import { FixFlowDialog } from './FixFlowDialog'

/** Encapsula o estado do diálogo de aplicar/reverter para qualquer página. */
export function useFixFlow() {
  const [state, setState] = useState<{ fix: FixPack | null; mode: 'apply' | 'revert'; open: boolean }>({ fix: null, mode: 'apply', open: false })
  return {
    start: (fix: FixPack, mode: 'apply' | 'revert') => setState({ fix, mode, open: true }),
    dialog: <FixFlowDialog fix={state.fix} mode={state.mode} open={state.open} onClose={() => setState((s) => ({ ...s, open: false }))} />,
  }
}
