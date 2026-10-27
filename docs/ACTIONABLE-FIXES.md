# Correções disponíveis e descobertas em investigação

Implementação da v0.5, 27/10/2026. A execução desta etapa começou em `bcabb60`, com working tree limpo. As implementações anteriores de hardware, arquivos e Resolution Intelligence já estavam no histórico; foram preservadas. Não houve commits, tags, push, alteração de versão ou operações na Steam real nesta etapa.

## Problemas encontrados

O sucesso do gerador dinâmico era usado como autorização visual para aplicar. Isso não comprovava que o planejador conseguia editar a configuração atual, encontrar o destino, atender os requisitos ou oferecer execução automática. O catálogo também confundia capacidade técnica da receita com aplicabilidade ao jogo selecionado. No Resolution Center havia nova consulta de compatibilidade sem contrato único com os cards. Alterações no disco/processos depois da pesquisa podiam deixar a interface desatualizada.

Além disso, configurações estruturadas eram recusadas genericamente, e receitas com artefato acompanhado por variável não tinham um caminho completo de composição. Os hashes de arquivos previstos por Smart Packs precisavam representar os bytes finais mesclados, e não somente o nome da operação.

## Fonte de verdade no Rust

`fix_engine::availability` converte o resultado do mesmo planejador usado pela prévia em um DTO compartilhado por descobertas, Resolution Plans e Smart Packs:

| Estado | Significado |
| --- | --- |
| `available` | Plano automático, pronto, com mudanças e fingerprint atual |
| `temporary` | Suporte existente; jogo/Steam abertos ou operação conflitante exigem intervenção e reavaliação |
| `investigation` | Não existe plano integralmente suportado ou validado |
| `incompatible` | Requisito de jogo, hardware ou software não atendido |
| `applied` | Alteração já presente; consultar histórico/verificação/restauração |

Cada impedimento tem código e explicação: escopo/fonte, receita inexistente ou alterada, hardware/Proton, dependência ou registro sem executor, artefato em revisão, destino/configuração inválidos, conflito, operação ativa, estado externo divergente, jogo aberto e Steam aberta. Uma linha de inicialização impossível de editar automaticamente não recebe `available` por existir uma alternativa manual. O planejador também consulta permissões efetivas dos diretórios de destino, backups e journal sem criar arquivos de sondagem; escrita, sincronização e espaço disponível continuam sujeitos a falhas tratadas pela transação.

`analyze_fix_findings` recalcula a aplicabilidade; o registro de receitas não serve como cache de autorização. Registros antigos sem o DTO carregam como investigação, preservando seu conteúdo. A interface envia o fingerprint da listagem a `prepare_fix_plan`; uma diferença exige reavaliação antes do consentimento. A aplicação continua revalidando contexto, fonte, destino e fingerprint. Nenhum texto externo recebe autoridade para executar comandos.

Uma operação pendente no mesmo AppID bloqueia novos planos mesmo quando altera outras chaves/arquivos; o teste cobre a pendência surgindo depois da listagem. O bloqueio pertence ao planejador e não impede recuperação ou rollback. Falhas ao atualizar Smart Packs/diagnósticos invalidam a disponibilidade anterior, inclusive de planos já desfeitos; dados do histórico permanecem acessíveis.

## Ampliação real do executor existente

Nova ação declarativa `set_dxvk_option`, integrada ao planejador, journal, escrita confinada, backup e rollback existentes. Não há executor paralelo.

O parser aceita uma instrução explícita como `Config/dxvk.conf: dxgi.maxFrameLatency = 1`, vinda de uma descoberta verificavelmente atribuída ao AppID. O caminho não é deduzido pelo nome do jogo. O diretório pai deve existir e pertencer à instalação resolvida pelo Game Discovery. A análise preserva as outras etapas da mesma instrução; uma dependência desconhecida bloqueia o conjunto.

O intérprete é determinístico e limitado, não um interpretador geral de prosa. Comandos/instaladores e condições reconhecidos junto da configuração deixam o procedimento inteiro em investigação. Não há suporte genérico para editar qualquer INI ou inferir destinos/etapas ausentes.

Classe inicial permitida:

| Chaves | Valores aceitos pelo FRX |
| --- | --- |
| `dxgi.enableHDR`, `dxgi.deferSurfaceCreation`, `d3d9.deferSurfaceCreation` | `True`, `False` |
| `dxgi.maxFrameLatency`, `d3d9.maxFrameLatency` | Inteiro de 0 a 16 |
| `dxgi.maxFrameRate`, `d3d9.maxFrameRate` | Inteiro de 0 a 360 |

Os nomes e tipos correspondem à [configuração oficial do DXVK](https://github.com/doitsujin/dxvk/blob/master/dxvk.conf), consultada nesta etapa. O FRX aceita um subconjunto conservador. Limitar FPS não representa melhoria universal; a documentação do DXVK recomenda limitadores externos quando disponíveis. Persistir HDR não prova suporte de tela, jogo ou execução.

O arquivo deve ser UTF-8, com no máximo 256 KiB e sintaxe reconhecida. Seções ambíguas e duplicatas das chaves alteradas são recusadas. Outras opções são preservadas. Symlinks, travessia, destino externo, alteração de identidade e divergência do conteúdo impedem execução. As alterações sobre o mesmo arquivo são reunidas em uma escrita. Um artefato e uma configuração que disputem o mesmo destino bloqueiam o conjunto.

Soluções podem combinar variáveis permitidas, configuração DXVK e artefatos de configuração já aceitos pela política do File Fix Engine. Downloads continuam sujeitos a procedência, versão, HTTPS/hosts/redirecionamentos, tamanho, checksum, formato e destinos. ZIP continua seletivo e é processado em staging. Falha na preparação não deixa metade da solução instalada; falha durante escrita usa recuperação transacional. Não se promete atomicidade entre arquivos.

Requisitos de AppID, hardware, driver e Proton acompanham a receita e seu fingerprint. Requisito obrigatório sem contexto verificável não vira compatibilidade presumida. Build do jogo exigido pela fonte precisa corresponder ao contexto disponível. Positividade de relatos nunca contorna a allowlist.

## Smart Packs e Performance Lab

Somente receitas com suporte validado participam da composição. Bloqueios operacionais temporários permanecem explicáveis, sem um botão de aplicação falso. Uma descoberta incompleta é excluída como unidade, sem retirar silenciosamente suas etapas; ela não bloqueia outras receitas independentes válidas.

O pacote completo passa novamente pelo planejador, incluindo conflitos entre arquivos. IDs, arquivos associados e snapshots de benchmark consideram o conteúdo mesclado previsto. Mudança de configuração modifica a identidade do pacote, impedindo reaproveitar evidência de outro conteúdo. Ranking, isolamento por jogo/hardware e regras de comparabilidade continuam existentes; nenhuma fixture vira benchmark real.

## Interface e Resolution Center

O layout, cores, mascotes e animações existentes foram preservados. As páginas do jogo e descobertas usam a mesma lista de correções disponíveis. Condições temporárias têm instrução e reavaliação. A pesquisa técnica conserva filtros, todos os jogos, detalhes e links diretos para a descoberta. O catálogo continua consultável sem prometer aplicação sem contexto.

O Resolution Center distingue problemas, soluções aplicáveis, informações em investigação e operações já registradas. Aplicação e restauração atualizam a análise. Respostas atrasadas não atravessam troca de jogo. Configuração aplicada continua significando apenas alteração persistida; a solução do sintoma exige a verificação apropriada.

## Limites de suporte

Continuam em investigação DLLs/binários sem política específica de confiança e reversão, instaladores Winetricks, registro Wine, troca automática de Proton, destino ausente/ambíguo, dependência desconhecida e instruções parcialmente entendidas. Mecanismos de atualização binária DLSS/FSR/PhysX não foram liberados por conveniência. O arquivo histórico do Batman continua não identificado. O FRX não inventa receitas quando nenhuma solução automática está disponível.

A disponibilidade é uma avaliação do instante da análise. Processos e arquivos podem mudar depois dela; prévia e execução revalidam. Falhas de rede, permissões ou disco ainda podem ocorrer e devem aparecer como erros, com preservação/recuperação da transação.

## Validação

Testes executados nesta etapa: `cargo test` com **400 aprovados, zero falhas, 7 ignorados**. A integração optativa `controlled_https_download_install_restart_and_rollback` foi executada separadamente e passou: TLS com CA local explicitamente configurada, rejeição de certificado sem confiança/hostname incorreto, download e hash, backup, aplicação, reinício e rollback. Nenhuma requisição dessa fixture sai de localhost. Os outros seis testes optativos de ambiente/rede não foram executados.

`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, TypeScript e build web passaram. **75 testes frontend passaram** em 11 arquivos. A cobertura adicional inclui fonte alterada, troca de jogo, receita inexistente, indisponibilidade técnica, condição temporária, falha de atualização e planos desfeitos sem permissão de reaplicar dados obsoletos. A recuperação da prévia obsoleta fecha e revalida a listagem, mantendo o fingerprint obrigatório desse fluxo.

Cancelar a importação de log conserva também o estado de validação; não transforma um relatório antigo em plano disponível após uma falha. Os últimos ajustes de recuperação/cancelamento receberam cobertura React/IPC. O build web mantém o aviso de chunk principal acima de 600 kB; isso não impediu compilação.

`npm run build` final passou com o código atualizado: binário Linux otimizado, `FRX_0.5.0_amd64.AppImage`, `FRX_0.5.0_amd64.deb` e `FRX-0.5.0-1.x86_64.rpm`. Os pacotes estão em `src-tauri/target/release/bundle/`, ignorado pelo Git; não foram instalados nem publicados. Versão 0.5.0 preservada. Log completo: `output/resolution-v05/actionable/tauri-build-final.log`.

Na janela Tauri real foram exercitados oito cenários com fontes, Steam e jogos **sintéticos**:

| Cenário | Resultado observado |
| --- | --- |
| Variável permitida | Prévia, confirmação, aplicação e rollback; wrapper `gamescope` preservado |
| Substituição de `dxvk.conf` | Backup, alteração, persistência após reinício e restauração |
| Solução composta | Variável + criação de `Config/dxvk.conf`; rollback remove somente o arquivo adicionado e a variável da operação |
| DLL de origem inadequada | Somente pesquisa técnica, sem botão Aplicar |
| Procedimento incompleto | Somente pesquisa técnica, sem aplicação parcial |
| Requisito Intel incompatível | Fora da lista de correções disponíveis |
| Jogo aberto | Marcador no `/proc` fictício: instrução de fechamento e reavaliação, sem botão Aplicar |
| Fonte modificada após listagem | Fingerprint rejeitado antes de qualquer escrita |

Oito cenários passaram durante a integração; após a revisão final do backend, o app foi reiniciado com o novo binário debug, conferindo os três journals desfeitos e uma nova prévia. Os arquivos originais da fixture foram restaurados integralmente, inclusive opções do segundo jogo. O arquivo adicionado foi removido, o cache de fonte de teste foi restaurado e o marcador de processo fictício foi removido. Backups e histórico permaneceram disponíveis.

As janelas foram conferidas em **1440×900 e 1024×760**. Capturas e resultados em `native-validation.json`/`native-integrity.json` ficam em `output/resolution-v05/actionable/`, já coberto pelo `.gitignore`. Esse diretório contém somente evidências locais de teste e não integra o Git. Não foi necessário ampliar o `.gitignore`.

Estado final do Git: HEAD `bcabb60` preservado, 49 arquivos rastreados modificados e seis novos (55 ao todo), índice vazio. `git diff --check` passou; nenhum artefato de build, log, backup ou instalação fictícia aparece na lista de alterações. A varredura de padrões de credenciais não encontrou correspondências nos arquivos alterados.

A validação confirma aplicação e persistência de configurações, não seu carregamento por um jogo real nem ganho de desempenho. Nenhum jogo real foi iniciado, nenhuma configuração real da Steam foi alterada e nenhum benchmark sintético foi inserido no histórico real. Somente os processos próprios da validação foram iniciados/encerrados.

## Arquivos alterados nesta etapa

Inventário do working tree para revisão; nenhum destes arquivos foi commitado nesta execução.

- `README.md`
- `docs/ACTIONABLE-FIXES.md`
- `docs/AUTOMATIC-FILE-FIX-ENGINE.md`
- `docs/FIX-ENGINE-DINAMICO.md`
- `docs/GAME-SPECIFIC-FIX-INTELLIGENCE.md`
- `docs/SMART-PACKS.md`
- `docs/VALIDACAO-v0.5.md`
- `src-tauri/src/commands/fix_engine.rs`
- `src-tauri/src/fix_engine/artifact_tests.rs`
- `src-tauri/src/fix_engine/availability.rs`
- `src-tauri/src/fix_engine/availability_tests.rs`
- `src-tauri/src/fix_engine/dynamic/analyzer.rs`
- `src-tauri/src/fix_engine/dynamic/extraction.rs`
- `src-tauri/src/fix_engine/dynamic/generator.rs`
- `src-tauri/src/fix_engine/dynamic/mod.rs`
- `src-tauri/src/fix_engine/mod.rs`
- `src-tauri/src/fix_engine/models.rs`
- `src-tauri/src/fix_engine/planner.rs`
- `src-tauri/src/fix_engine/security.rs`
- `src-tauri/src/fix_engine/structured_config_tests.rs`
- `src-tauri/src/fix_engine/tests.rs`
- `src-tauri/src/fixes/aggregation.rs`
- `src-tauri/src/fixes/tweaks.rs`
- `src-tauri/src/resolution/acceptance_tests.rs`
- `src-tauri/src/resolution/mod.rs`
- `src-tauri/src/resolution/models.rs`
- `src-tauri/src/resolution/plans.rs`
- `src-tauri/src/smart_packs/benchmark.rs`
- `src-tauri/src/smart_packs/composition.rs`
- `src-tauri/src/smart_packs/conflicts.rs`
- `src-tauri/src/smart_packs/discovery.rs`
- `src-tauri/src/smart_packs/lifecycle.rs`
- `src-tauri/src/smart_packs/mod.rs`
- `src-tauri/src/smart_packs/models.rs`
- `src-tauri/src/smart_packs/tests.rs`
- `src/components/fix/ActionableFindings.tsx`
- `src/components/fix/ApplyFixDialog.tsx`
- `src/components/fix/FindingCard.tsx`
- `src/components/fix/RecipesPanel.tsx`
- `src/components/fix/TechnicalFindingsBrowser.tsx`
- `src/components/fix/useAssessments.ts`
- `src/components/resolution/ResolutionCenter.tsx`
- `src/components/smart/PackTest.tsx`
- `src/components/smart/SmartPacksPanel.tsx`
- `src/pages/FindingsPage.tsx`
- `src/pages/RealHome.tsx`
- `src/pages/SteamGameDetail.tsx`
- `src/services/fixEngine.ts`
- `src/test/dynamicFix.test.tsx`
- `src/test/gameSpecific.test.tsx`
- `src/test/resolution.test.tsx`
- `src/test/smartPacks.test.tsx`
- `src/types/fixEngine.ts`
- `src/types/resolution.ts`
- `src/types/smartPacks.ts`
