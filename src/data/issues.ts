import type { Issue, Source } from '@/types'

/** Problemas conhecidos — DEMONSTRATIVOS. Referências de fontes são ilustrativas. */

const pdb = (ref: string): Source => ({ kind: 'protondb', label: 'ProtonDB', ref })
const gh = (ref: string): Source => ({ kind: 'github', label: 'GitHub', ref })
const umu = (ref: string): Source => ({ kind: 'umu-protonfixes', label: 'UMU-Protonfixes', ref })
const lab = (ref: string): Source => ({ kind: 'frx-lab', label: 'FRX Lab', ref })
const wiki = (ref: string): Source => ({ kind: 'wiki', label: 'Wiki da comunidade', ref })

export const issues: Issue[] = [
  {
    id: 'ferrous-stutter',
    gameId: 'ferrous-drift',
    title: 'Engasgos ao compilar shaders',
    description: 'Quedas bruscas de FPS nas primeiras voltas de cada pista enquanto os shaders são compilados em tempo real.',
    severity: 'alta',
    symptom: 'desempenho',
    sources: [pdb('84 relatos semelhantes (demo)'), gh('issue #—— em um tracker de compatibilidade (demo)')],
  },
  {
    id: 'ferrous-fsync',
    gameId: 'ferrous-drift',
    title: 'Uso de CPU elevado na sincronização',
    description: 'Threads de sincronização disputam CPU em cenas com muitos carros na tela.',
    severity: 'media',
    symptom: 'desempenho',
    sources: [pdb('31 relatos (demo)')],
  },
  {
    id: 'lantern-flicker',
    gameId: 'lanterna-oca',
    title: 'Cintilação em reflexos de água',
    description: 'Reflexos piscam em algumas GPUs ao usar a tradução D3D12 → Vulkan.',
    severity: 'media',
    symptom: 'video',
    sources: [pdb('22 relatos (demo)'), umu('entrada de exemplo para o jogo (demo)')],
  },
  {
    id: 'lantern-intro',
    gameId: 'lanterna-oca',
    title: 'Vídeo de introdução em tela preta',
    description: 'A cena inicial não é reproduzida por falta de codecs de mídia no prefixo.',
    severity: 'baixa',
    symptom: 'video',
    sources: [umu('regra de mídia de exemplo (demo)')],
  },
  {
    id: 'ashen-eac',
    gameId: 'ashen-choir',
    title: 'Modo online não conecta',
    description: 'O runtime do anti-cheat não é carregado, impedindo partidas cooperativas.',
    severity: 'alta',
    symptom: 'rede',
    sources: [pdb('140 relatos (demo)'), wiki('página de anti-cheat (demo)')],
  },
  {
    id: 'ashen-cutscene',
    gameId: 'ashen-choir',
    title: 'Fechamento inesperado em cutscenes',
    description: 'O jogo fecha ao entrar na segunda cutscene do capítulo 3.',
    severity: 'alta',
    symptom: 'crash',
    sources: [gh('relato de exemplo (demo)'), pdb('57 relatos (demo)')],
  },
  {
    id: 'ashen-nvapi',
    gameId: 'ashen-choir',
    title: 'Upscaler indisponível no menu',
    description: 'A opção de upscaling da GPU aparece desabilitada nas configurações gráficas.',
    severity: 'baixa',
    symptom: 'video',
    sources: [pdb('19 relatos (demo)')],
  },
  {
    id: 'starfall-hdr',
    gameId: 'starfall-outpost',
    title: 'HDR não aparece nas opções',
    description: 'O jogo não detecta uma saída HDR na sessão atual.',
    severity: 'baixa',
    symptom: 'video',
    sources: [wiki('guia de HDR (demo)')],
  },
  {
    id: 'glass-audio',
    gameId: 'glasswing',
    title: 'Chiado e atraso no áudio',
    description: 'Estalos audíveis e latência que tira a música do ritmo das notas.',
    severity: 'alta',
    symptom: 'audio',
    sources: [pdb('12 relatos (demo)'), lab('teste interno simulado')],
  },
  {
    id: 'dune-video',
    gameId: 'dune-courier',
    title: 'Vídeos in-game não carregam',
    description: 'Telas de transmissão dentro do jogo ficam com padrão de teste colorido.',
    severity: 'media',
    symptom: 'video',
    sources: [umu('regra de mídia de exemplo (demo)'), pdb('46 relatos (demo)')],
  },
  {
    id: 'tide-controller',
    gameId: 'tidebreak-tactics',
    title: 'Botões do controle trocados',
    description: 'Confirmar e cancelar aparecem invertidos em controles genéricos.',
    severity: 'baixa',
    symptom: 'input',
    sources: [pdb('9 relatos (demo)')],
  },
]

export const issueById = (id: string) => issues.find((i) => i.id === id)
