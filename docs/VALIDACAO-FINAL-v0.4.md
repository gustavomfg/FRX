# Revisão final do FRX v0.4 — Smart Packs

Data: 26/10/2026. Base: `7c1e5eb`, branch `main`, inicialmente limpa e sincronizada com `origin/main`. O fluxo automático já estava nos commits `f36a79e`, `b7b450a` e `7c1e5eb`; esta revisão o corrige e completa, sem recriá-lo nem iniciar a v0.5.

## Situação da validação real

**A validação no Batman ainda não ocorreu.** O usuário informou que não pode jogar nesta sessão e pediu que a Steam real não fosse alterada. Nenhum jogo foi iniciado, nenhuma captura A1/A2/B1/B2 foi registrada e nenhuma configuração real da Steam ou de outro jogo foi escrita pelo agente.

As estimativas informais fornecidas pelo usuário não foram convertidas em benchmarks. Não há valores medidos novos de FPS, lows, P99 ou stuttering para comparar aqui:

| Captura de Batman: Arkham City | Configuração | Resultado nesta revisão |
|---|---|---|
| A1 | Configuração original, sem correções do FRX | Pendente |
| A2 | Igual a A1, mesmo trecho | Pendente |
| B1 | `PROTON_NO_D3D11=1`, mesmas condições | Pendente |
| B2 | Igual a B1, mesmo trecho | Pendente |

Portanto, não há conclusão sobre ganho ou perda real de FPS nem sobre o ranking desta máquina. O rollback na instalação real também permanece pendente.

## Problemas encontrados e correções

- **Associação só no fim:** a captura agora registra a configuração no início e confere novamente ao terminar. Mudanças nas variáveis/operações invalidam a associação para o ranking. O fluxo guiado envia `expectedPackId`, conferido antes de preparar e antes de armar; um pacote inesperado ou operação pendente gera erro recuperável.
- **Estado incerto usado como evidência:** sessões interrompidas, impróprias ou com snapshot não confirmado são preservadas, mas excluídas. Passos manuais pendentes e operações em recuperação deixam a configuração incerta, inclusive quando nenhuma correção aparece como ativa.
- **Contextos diferentes misturados:** diferenças conhecidas de CPU, GPU, Proton, driver, kernel e condições gráficas excluem a sessão da comparação controlada. Os metadados e a evidência anterior continuam no histórico; a descoberta não é removida nem proibida globalmente.
- **Dados sintéticos exportados como captura:** existe `source: synthetic`; dados assim marcados, avisos `synthetic`/`fixture` e IDs legados `s-fixture-*` são excluídos. O teste de aceitação exporta suas fixtures com o marcador correto. A interface as identifica como sintéticas. Não é possível provar a autenticidade de todo CSV externo só pelo formato: o marcador protege os dados de teste identificados.
- **Piora de estabilidade ocultada pelo FPS:** a conclusão considera também 0,1% low, P99 e picos. Melhora em FPS com piora de outra métrica vira sinal misto, com valores visíveis. O perfil de desempenho não elege essa troca como vencedor. Picos que saem de zero aparecem em valores absolutos, com percentual indisponível.
- **Afirmação excessiva:** a mensagem “Comprovado” foi substituída por observação em sessões repetidas. A avaliação explica que duas sessões por configuração são um início e não provam causalidade.
- **Resultado anterior reaparecendo:** iniciar/repetir um teste não reutiliza a última captura concluída. Respostas antigas não substituem o relatório de outro jogo/perfil. Erros de avaliação aparecem e permitem tentar novamente. A referência recebe seu próprio rótulo, o jogo fica fixo e o início manual vem selecionado no fluxo guiado.
- **Preparação exigindo configuração fora do FRX:** para Vulkan, o assistente oferece uma receita interna `frx.capture.<appid>` pelo mesmo Fix Engine, com prévia, confirmação, backup e rollback. Ela gera apenas `MANGOHUD=1` e o caminho próprio do FRX, sem aceitar caminhos da interface nem ampliar a allowlist das descobertas. Quando a configuração já está correta, aparece “Aguardar o jogo”. Essa preparação não altera a identidade do Smart Pack nem é desfeita junto com uma correção de desempenho.
- **Arquivos próprios:** o arquivo de configuração usa a escrita atômica existente, pastas privadas e recusa de links simbólicos no destino/pastas de captura.

A preparação automática cobre a camada Vulkan, incluindo jogos Proton que usam DXVK/VKD3D. OpenGL e Steam Flatpak mantêm a alternativa manual e precisam de validação própria. Nenhuma configuração é aplicada com a Steam real aberta; o usuário precisa fechá-la normalmente para usar a escrita automática.

## Verificações automatizadas atuais

| Verificação | Resultado |
|---|---|
| `cargo fmt --check` | Passou |
| `cargo clippy --all-targets -- -D warnings` | Passou |
| `cargo test` | **223 aprovados**, 0 falhas, **6 ignorados** |
| `npm test` | **42 aprovados**, 8 arquivos |
| `npm run build:web` | Passou, incluindo TypeScript |
| `npm run build` (Tauri Linux) | Passou: AppImage (110,05 MiB), DEB e RPM (4,32 MiB cada) |

Os seis ignorados dependem de rede, dados locais ou procedimentos específicos: `real_fix_search`, `real_log`, `real_system_scan`, `real_dynamic_end_to_end`, `real_smart_packs_end_to_end` e `acceptance_test_performance_batman`. Este último é uma aceitação com fixtures, não um teste de FPS real do Batman.

A cobertura nova verifica preparação com backup/rollback seletivo após reinício, preservação de parâmetros posteriores, configuração alterada/pendente, pack inesperado, configuração existente com aspas/wrappers, links simbólicos, rejeição de sessões sintéticas/interrompidas/não confirmadas, troca de contexto de software, melhora repetida, trade-offs de frametime e picos a partir de zero. Na interface, cobre rótulo e preset do baseline, jogo fixo/início manual, resultado antigo, cancelamento e repetição após erro de avaliação.

Avisos não bloqueantes: dimensões nulas de Recharts no DOM de teste e chunk web acima de 600 kB. O detector de interface não apontou achados determinísticos nos componentes revisados; isso não substitui inspeção visual.

## Verificação no aplicativo Tauri

Foi usado o aplicativo Tauri real, com HOME/XDG e uma Steam fictícia separados, `/proc` de teste no debug e nenhum processo real da Steam encerrado. O cache copiado contém somente descobertas públicas. Não foram copiados benchmarks pessoais para produzir conclusões.

Percorrido por automação de acessibilidade no Hyprland, inicialmente com backend Xwayland:

1. Biblioteca da instalação fictícia com Batman → Smart Packs, baseline sem medições e candidato de Direct3D 11.
2. Capturar referência → formulário com rótulo de referência, início manual e jogo fixo.
3. Preparar sessão → configurar MangoHud pelo FRX → prévia → confirmação → operação aplicada com backup na pasta isolada.
4. Voltar ao assistente: configuração reconhecida e botão “Aguardar o jogo”, sem colagem.
5. Aguardar, sem abrir o jogo → cancelar: **zero sessões persistidas** e configuração sem autostart pendente.
6. Testar desempenho sem baseline → pular referência explicitamente → revisar e aplicar Direct3D 11 na instalação fictícia → avanço para capturar.
7. Restaurar estado anterior → prévia do rollback → confirmação. A variável `PROTON_NO_D3D11` foi removida; MangoHud, `gamescope`, argumentos preexistentes e as opções de outro jogo fictício foram preservados. O histórico marcou a operação como `rolled-back`.

Foram inspecionadas janelas de 1200×800 e 1024×760. Artefatos de inspeção e os resultados de disco ficam em `output/playwright/v04-final/`, ignorado pelo Git. Não se usa o hash do arquivo inteiro como prova de restauração: a verificação considera as opções controladas e a preservação dos demais campos.

Depois, o binário **release final** abriu com backend **Wayland nativo** no Hyprland, usando os mesmos dados isolados. O Histórico carregou as duas operações persistidas (preparação aplicada e correção desfeita). A prévia do rollback da preparação abriu e foi cancelada, sem escrita. O release usa o `/proc` real; a variável de teste continua restrita ao debug.

O navegador também abriu o modo demonstrativo em Playwright e confirmou a identificação Demo e ausência de overflow horizontal em 1024 px. Isso é apenas uma verificação adicional do navegador, não uma medição ou validação do jogo.

## Protocolo preparado para a próxima sessão

1. Abrir o FRX e selecionar Batman: Arkham City GOTY em **Fix Packs → Smart Packs**. Escolher um trecho repetível; registrar resolução, preset, VSync, limite de FPS e duração (por exemplo, 60 s). Manter Proton, driver e demais condições iguais.
2. Selecionar **Baseline → Capturar referência**. Se houver correções do FRX, usar a restauração com prévia e confirmação antes. Para preparar automaticamente o MangoHud Vulkan, fechar jogo e Steam normalmente, escolher **Configurar pelo FRX (Vulkan)** e confirmar a prévia. Nenhum comando externo é necessário nesse caminho suportado.
3. Escolher início manual, preparar a sessão, clicar **Aguardar o jogo** e **Abrir o jogo pela Steam**. Neste ponto o usuário joga: aquecer o cenário, chegar ao trecho escolhido e iniciar por `Shift esquerdo + F2` ou pelo controle do FRX. Repetir a mesma sequência durante a duração completa.
4. Salvar A1 e A2, reabrindo o jogo quando as condições de captura exigirem. O FRX associa as sessões ao baseline automaticamente. Conferir os avisos e os rótulos para distinguir as repetições.
5. Fechar jogo e Steam normalmente. No candidato com `PROTON_NO_D3D11=1`, escolher **Testar desempenho → Revisar e aplicar**. Revisar e confirmar somente essa mudança. Fazer B1 e B2 no mesmo trecho, com o mesmo protocolo. As duas primeiras capturas por configuração são apenas o ponto inicial.
6. Ao concluir cada captura, o FRX avalia automaticamente a sessão: médias/medianas comparadas, lows disponíveis, P99, picos, diferenças absolutas/percentuais, limiares, condições e número de sessões. Com uma sessão de algum lado, o resultado é preliminar. Com repetições comparáveis, a evidência entra no ranking por perfil, acompanhada de limitações.
7. Se houver regressão ou trade-off, revisar a tabela e usar **Restaurar configuração anterior**, confirmando cada passo. Fechar o jogo e a Steam normalmente para o FRX restaurar sem edição externa. Conferir o histórico e a configuração ativa após reiniciar o FRX.
8. Se houver aquecimento, variação temporal ou dispersão alta, fazer novas repetições em sequência alternada A/B, sempre com restauração e confirmação entre configurações. Nunca transformar uma captura inconclusiva em recomendação definitiva.

## Pendências para encerrar a validação real

- A1, A2, B1 e B2 reais, com participação e autorização do usuário.
- Inspeção das condições/qualidade e do ranking decorrente dessas medições, sem inferir causalidade absoluta.
- Aplicação e rollback na instalação real, confirmados no app, com preservação das opções relevantes e persistência após reinício.
- Verificação do efeito do pacote no jogo e da repetibilidade do cenário; nenhum teste automatizado substitui isso.

A implementação pode ser revisada e commitada localmente com esses limites. **A v0.4 ainda não pode ser declarada validada com capturas reais do Batman.** Não houve push, tag, release publicada ou alteração de versão nesta revisão.


## Arquivos desta revisão

- Rust/IPC: `src-tauri/src/commands/performance.rs`.
- Fix Engine: `src-tauri/src/fix_engine/{mod,planner,tests}.rs`.
- Performance Lab: `src-tauri/src/performance/{capture,comparison,models}.rs`.
- Smart Packs: `src-tauri/src/smart_packs/{benchmark,evaluation,evidence,mod,models,ranking,tests}.rs`.
- Interface: `src/components/performance/{CaptureFlow,HistoryTable,SessionView}.tsx`, `src/components/smart/{PackTest,SessionPackResult,SmartPacksPanel}.tsx` e `src/components/ui/Controls.tsx`.
- Modelos e testes React: `src/types/{performance,smartPacks}.ts`, `src/test/smartPacks.test.tsx`.
- Documentação: `docs/SMART-PACKS.md`, `docs/VALIDACAO-SMART-PACKS.md` e este relatório.

Artefatos de build, capturas de tela, diretórios de teste, logs, backups e dados isolados ficaram fora do histórico, nos caminhos já ignorados pelo projeto. A revisão não encontrou segredos nos arquivos pendentes. Nenhuma alteração não relacionada existia no início nem foi incluída.


## Ajuste posterior — triagem automática de sugestões (27/10/2026)

O usuário esclareceu que queria a separação preventiva de alternativas com possível perda de desempenho, sem precisar testar cada sugestão. O FRX já tinha descrições de custo, mas não as usava como categoria própria antes dos benchmarks.

A análise preventiva agora chega às descobertas, ao catálogo, à prévia e aos Smart Packs pelo mesmo classificador Rust. WineD3D, desativação de sincronizações, fallback de APIs e limitadores de FPS recebem explicações específicas. A seção **Risco de perda de desempenho** é independente do ranking medido; sem evidência local suficiente, essas alternativas não são sugeridas como otimizações, não entram em combinações automáticas e não geram uma sequência de testes recomendados. A ação disponível é revisar seu uso para compatibilidade. Impacto ainda desconhecido também não vira promessa de FPS.

Não se apaga nem proíbe uma correção globalmente. Medições locais já existentes continuam sendo usadas com seu contexto; o aviso preventivo permanece visível mesmo quando dados comparáveis permitem avaliar um caso específico. Nenhuma porcentagem nova foi inferida de um relato.

Verificações desta alteração:

- `cargo fmt --check` e Clippy com `-D warnings`: passaram.
- Rust: **228 aprovados, 6 ignorados**.
- Front-end: **43 aprovados**; TypeScript e build web passaram.
- Build Tauri Linux: AppImage, DEB e RPM gerados.
- Tauri com instalação fictícia, sem sessões: WineD3D e Direct3D 11 apareceram em **Risco de perda**, sem próximo teste recomendado nem botão de testar nesses cards. O histórico anterior foi preservado, com o novo campo aditivo `preventiveRisks`. A interface foi inspecionada em 1200×800 e 1024×760.

Os testes de transação e rollback passaram a usar ações sem esse risco nas fixtures de composição; os testes originais foram mantidos. A nova cobertura verifica triagem independente de benchmarks, conservação da receita aplicável, ausência de pedidos de teste para custos conhecidos, persistência sem repetir eventos e ausência de promessa de FPS para relatos de compatibilidade.

Esta verificação não iniciou jogos nem alterou a Steam real. Artefatos visuais e logs estão em `output/playwright/v04-performance-risk/`, fora do histórico. A versão do aplicativo continua 0.4.0; foram atualizadas apenas as versões internas de metodologia e conhecimento para recomputar o cache automaticamente.
