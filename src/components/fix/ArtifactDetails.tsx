import { Download, ExternalLink, FileBox, TriangleAlert } from 'lucide-react'
import type { ArtifactCandidate } from '@/types/fixes'
import { openExternal } from '@/services/opener'
import { fmtNum } from '@/lib/format'
import { HardwareRequirementsSummary } from './HardwareContext'

export function artifactBytes(bytes: number) {
  return bytes >= 1024 * 1024 ? `${fmtNum(bytes / (1024 * 1024), 2)} MiB` : bytes >= 1024 ? `${fmtNum(bytes / 1024, 1)} KiB` : `${fmtNum(bytes)} bytes`
}

export function ArtifactBadge({ artifacts }: { artifacts?: ArtifactCandidate[] }) {
  if (!artifacts?.length) return null
  return <span className="inline-flex items-center gap-1.5 rounded-xs bg-accent-soft px-2 py-1 text-[12px] font-medium text-accent"><FileBox className="size-3.5" aria-hidden /> Correção por arquivo · download necessário</span>
}

/** Immutable publisher metadata from the core; the UI never chooses a path or URL to install. */
export function ArtifactDetails({ artifacts }: { artifacts?: ArtifactCandidate[] }) {
  if (!artifacts?.length) return null
  return (
    <section aria-label="Arquivos externos da correção" className="space-y-4 text-[12.5px] text-fg-2">
      <p className="flex items-center gap-2 font-medium text-fg"><Download className="size-4 text-fg-3" aria-hidden /> Download e instalação</p>
      {artifacts.map((artifact) => (
        <div key={artifact.id} className="space-y-3 border-t border-line pt-3">
          <div className="flex flex-wrap items-start justify-between gap-2">
            <p className="min-w-0 break-all font-medium text-fg">{artifact.assetName}</p>
            <span className={artifact.trust === 'origin-and-integrity' ? 'text-fg-2' : 'text-caution'}>{artifact.trust === 'origin-and-integrity' ? 'Origem e hash identificados' : artifact.trust === 'partial' ? 'Requer revisão adicional' : 'Origem não confiável'}</span>
          </div>
          <dl className="space-y-1">
            <div className="flex flex-wrap gap-x-2"><dt className="text-fg-3">Projeto:</dt><dd className="min-w-0 break-all">{artifact.repository} · {artifact.publisher ?? 'mantenedor não identificado'}</dd></div>
            <div className="flex flex-wrap gap-x-2"><dt className="text-fg-3">Release:</dt><dd>{artifact.releaseTag || 'Versão não identificada'}</dd></div>
            <div className="flex flex-wrap gap-x-2"><dt className="text-fg-3">Download:</dt><dd>{artifactBytes(artifact.sizeBytes)} · {artifact.format === 'zip' ? 'ZIP, extração seletiva' : artifact.format === 'file' ? 'arquivo individual' : 'formato não confirmado'}</dd></div>
            <div className="flex flex-wrap gap-x-2"><dt className="text-fg-3">Plataforma / arquitetura:</dt><dd>{artifact.platform ?? 'não informada'} · {artifact.architecture ?? 'não informada'}</dd></div>
            <div className="space-y-1"><dt className="text-fg-3">Origem do arquivo:</dt><dd className="break-all font-mono text-[11.5px]">{artifact.assetUrl}</dd></div>
            <div className="space-y-1"><dt className="text-fg-3">SHA-256 publicado:</dt><dd className="break-all font-mono text-[11.5px] select-all">{artifact.sha256 ?? 'Não disponível; aplicação automática depende de integridade verificável.'}</dd></div>
            {artifact.signatureUrl && <div className="flex flex-wrap gap-x-2"><dt className="text-fg-3">Assinatura publicada:</dt><dd className="break-all">{artifact.signatureUrl}</dd></div>}
            {artifact.license && <div className="flex flex-wrap gap-x-2"><dt className="text-fg-3">Licença informada:</dt><dd>{artifact.license}</dd></div>}
          </dl>
          <HardwareRequirementsSummary requirements={artifact.requirements.hardware} />
          {artifact.files.length ? <ul className="space-y-2" aria-label={`Destinos de ${artifact.assetName}`}>
            {artifact.files.map((file) => <li key={`${file.sourcePath ?? ''}:${file.targetPath}`}>
              <p>{file.replace ? 'Substituir com backup:' : 'Adicionar:'} <code className="break-all font-mono text-[12px] text-fg">{file.targetPath}</code></p>
              {file.sourcePath && <p className="text-fg-3">Somente o membro <code className="break-all font-mono text-[11.5px]">{file.sourcePath}</code> será extraído.</p>}
              {file.sha256 && <p className="break-all font-mono text-[11.5px] text-fg-3">SHA-256 do arquivo: {file.sha256}</p>}
            </li>)}
          </ul> : <p className="text-caution">Destino não confirmado. O FRX mantém a descoberta para revisão.</p>}
          {!!artifact.reasons.length && <ul className="space-y-1 text-caution">{artifact.reasons.map((reason) => <li key={reason} className="flex gap-2"><TriangleAlert className="mt-0.5 size-3.5 shrink-0" aria-hidden />{reason}</li>)}</ul>}
          {artifact.trust !== 'origin-and-integrity' && (
            <div className="space-y-1.5 rounded-md border border-line bg-bg/50 px-3.5 py-3">
              <p className="font-medium text-fg">Se você quiser instalar por conta própria</p>
              <p>O FRX não baixa nem instala este arquivo. Se decidir usá-lo, faça à mão:</p>
              <ol className="list-decimal space-y-1 pl-5">
                <li>Abra a release original (botão abaixo) e baixe o arquivo {artifact.assetName}.</li>
                <li>Confira o SHA-256 publicado com o arquivo que você baixou.</li>
                <li>Copie apenas para os destinos listados acima. Se não houver destino, siga a documentação do projeto.</li>
              </ol>
              <p className="text-fg-3">O FRX não reconhece nem verifica o que você copiar à mão.</p>
            </div>
          )}
          <div className="flex flex-wrap gap-x-4 gap-y-2 text-fg-3">
            <button type="button" className="inline-flex items-center gap-1 hover:text-accent" onClick={() => void openExternal(artifact.releaseUrl).catch(() => {})}>Ver release original <ExternalLink className="size-3" aria-hidden /></button>
            <button type="button" className="inline-flex items-center gap-1 hover:text-accent" onClick={() => void openExternal(artifact.recommendationSource.url).catch(() => {})}>Origem da recomendação <ExternalLink className="size-3" aria-hidden /></button>
          </div>
        </div>
      ))}
      <p className="text-[12px] text-fg-3">O checksum verifica o conteúdo em relação à referência publicada. Ele não comprova que uma DLL ou outro código é benigno, nem que a correção melhora o FPS.</p>
    </section>
  )
}
