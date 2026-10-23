import { useNavigate } from 'react-router-dom'
import { Page } from '@/layouts/Page'
import { EmptyState } from '@/components/ui/EmptyState'
import { Button } from '@/components/ui/Button'

export function NotFoundPage() {
  const navigate = useNavigate()
  return (
    <Page>
      <EmptyState
        pose="confused"
        title="Essa fase não existe"
        body="O Pim procurou em todos os cantos e não encontrou esta página."
        action={
          <Button variant="primary" onClick={() => navigate('/')}>
            Voltar ao início
          </Button>
        }
        className="min-h-[70vh]"
      />
    </Page>
  )
}
