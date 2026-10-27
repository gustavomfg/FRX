# Validação — Hardware Intelligence e Automatic File Fix Engine

Data: 27/10/2026. Versão do aplicativo preservada em **0.4.0**. Trabalho disponível no diretório de trabalho para revisão, sem commit, tag ou push.

## Estado inicial e preservação

O repositório `/home/g/Documentos/FRX` estava limpo, na branch `main`, sincronizada com `origin/main`, em `b28d237` (`docs: document game-specific fix intelligence`). O histórico recente já continha `e3c6199` (interface por jogo) e `e61cd70` (isolamento do núcleo). Não havia AGENTS.md ou CLAUDE.md no projeto ou nos ancestrais inspecionados. Não houve reset, clean, descarte de alterações ou mudança de versão.

O Fix Engine transacional, descoberta, Smart Packs, isolamento por AppID, captura MangoHud e componentes visuais existentes foram estendidos, sem novo executor ou redesign.

## 1. GPUs, drivers e software

`EnvironmentContext` separa inventário de GPU efetivamente observada. IDs e endereço PCI, modelo/chip, VRAM disponível, módulo/versão do driver, CPU/arquitetura, sessão gráfica, Proton/origem e ICDs instalados são preservados sem inventar dados ausentes. Condições de sessão existentes conservam jogo/build, CPU, distribuição/kernel e configurações gráficas conhecidas.

O CSV do MangoHud pode confirmar um renderer somente quando corresponde de forma única ao inventário. Duas placas idênticas continuam ambíguas. GPU provável do desktop e ICD instalado não viram GPU/driver Vulkan usados pelo jogo. [Modelo completo](HARDWARE-FIX-INTELLIGENCE.md).

## 2. Recomendações por jogo e ambiente

Requisitos de fabricante, PCI, modelo, driver, Proton e arquitetura são avaliados explicitamente. Incompatível, desconhecido e compatível são estados diferentes. Relatos positivos/negativos de outro ambiente permanecem na origem, mas não promovem nem penalizam o contexto atual.

Ranking e avaliação de uma sessão selecionam sua própria coorte, inclusive quando outro hardware possui um baseline mais recente. Os testes isolados mantêm melhoria NVIDIA, regressão AMD e ausência de evidência Intel independentes para a mesma ação no mesmo jogo. Esses resultados são fixtures, não benchmarks do computador do usuário.

## 3. Identificação de correções por arquivo

A descoberta conserva separadamente recomendação e origem do asset. Identifica release/ID/tag, mantenedor, asset/ID/tamanho/URL/hash, formato, versão, plataforma/arquitetura, requisitos e destinos documentados. Instruções estritas ou manifesto genérico do publicador geram receita dinâmica sem cadastro JSON do desenvolvedor por jogo.

A busca por nome só localiza referências; não prova AppID. Comentários não escolhem livremente comandos ou destinos. Instruções incompletas ou com etapas desconhecidas ficam em revisão.

## 4. Fontes integradas

As fontes GitHub e UMU existentes passam a localizar referências de GitHub Releases e seus metadados públicos. Os repositórios reconhecidos e o protocolo suportado estão na [investigação](BATMAN-PHYSX-INVESTIGATION.md#contrato-de-descoberta-de-arquivos-suportado). Não há busca geral irrestrita de binários, espelhamento de downloads, execução de Python externo ou instalação de dependências do sistema.

## 5. Procedência e integridade

O núcleo exige projeto reconhecido, escopo correto, URL canônica, publicador/release/asset identificados, digest e instruções completas. Reconsulta a release por ID e regenera o candidato antes do payload, recusando qualquer divergência. HTTPS usa verificação TLS normal e política de hosts/redirecionamentos; SHA-256 confere a referência publicada.

Isso não certifica benignidade ou melhoria. URLs de assinaturas podem ser identificadas, mas assinatura criptográfica ainda não é verificada. Detalhes e limites estão na [política](AUTOMATIC-FILE-FIX-ENGINE.md#política-de-confiança).

## 6. Arquivos realmente aplicáveis

Configurações UTF-8 INI/CFG/CONF/JSON/TXT, diretas ou membros selecionados de ZIP, sujeitas à gramática de dados e limites. Substituições exigem hash do original; patches conhecidos fazem substituição literal inequívoca sobre hash exato e validam o resultado. Essa última ação tipada ainda não é inferida de prosa externa.

## 7. O que permanece em revisão

DLL/EXE/ELF, scripts, XML/TAR, modificações de prefixos/Proton, winetricks e procedimentos incompletos não são automatizados. Metadados de uma release legítima não liberam seus binários. Arquiteturas e faixas de versão não suportadas não são adivinhadas. Arquitetura da CPU não comprova arquitetura do jogo: exigência de x86/x86_64 no artefato permanece em revisão sem essa confirmação. Procedimentos de vários assets ainda exigem revisão, sem aplicar um fragmento isolado.

## 8. Instalação

A prévia mostra jogo, ambiente e limitações, fonte/versão, arquivos/tamanho/hash, destinos, substituições e plano de recuperação. Confirmação inclui consentimento de download. O Engine revalida o plano, prepara todos os artefatos em staging, valida conteúdo e cria todos os backups antes da escrita.

Raiz e pais são percorridos com descritores sem seguir symlinks. Há proteção contra traversal, ZIP bombs, links, destinos concorrentes e escrita em outro jogo. Steam/jogo são rechecados após download e antes das gravações; não são fechados pelo FRX. Atomicidade é por arquivo, com journal para coordenar múltiplas alterações.

## 9. Backup, rollback e recuperação

Backups verificados e intenção `writing` persistem antes da troca. Rollback tem prévia, confirmação, validação de hashes e preservação de alterações externas. Recomeçar uma tentativa usa novos nomes de backup e não apaga cópias anteriores.

A revisão final encontrou e corrigiu: symlink na raiz/ancestral antes da primeira prévia; segunda alteração concorrente durante reversão de troca; início de Steam/jogo durante rollback ou compensação; colisão com backup de tentativa interrompida; falso encerramento de recuperação quando conteúdo externo ainda está retido; e requisitos de build/plataforma do artefato armazenados sem checagem final independente. Neste último caso, `manualRecoveryRequired` mantém a pendência e a localização do temporário após reinício. Não há liberação automática dessa recuperação manual nesta fase.

## 10. Smart Packs e benchmarks

Pacotes podem misturar variáveis e configurações por arquivo suportadas na mesma operação. Conflitos de arquivos, requisitos conjuntos e política são revalidados. Uma ação não validada impede a receita completa; não se aplica só a parte conveniente.

Snapshots incluem destinos, versões e hashes realmente instalados. Atualização de artefato muda a identidade; staging não é benchmark verificado. Mudanças de arquivos/build, AppID, GPU, driver e Proton impedem reutilização incompatível. Históricos antigos não são apagados; campos novos têm defaults e packs legados só de variáveis mantêm IDs.

## 11. Investigação Batman/PhysX

A [issue GE-Proton #196](https://github.com/GloriousEggroll/proton-ge-custom/issues/196) aponta para [SveSop/nvidia-libs](https://github.com/SveSop/nvidia-libs). A [release v1.0.3](https://github.com/SveSop/nvidia-libs/releases/tag/v1.0.3) fornece metadados/digests, mas envolve bibliotecas binárias, TAR e procedimentos para prefixo/Proton sem mapeamento suficiente para instalar um arquivo no Arkham City do usuário. O payload não foi baixado.

O [Proton CachyOS](https://github.com/CachyOS/proton-cachyos/blob/cachyos_main/README.md) documenta requisitos e ressalvas NVIDIA/PhysX. A dica de `PhysXDevice.dll` no BmLauncher é para Arkham Asylum, não Arkham City. **O arquivo histórico exato não foi confirmado**. Não foi criado um pack PhysX fictício nem convertido FPS informal em medição. [Análise e fontes completas](BATMAN-PHYSX-INVESTIGATION.md).

## 12. Por que não aparecia antes

A busca anterior priorizava AppID no título e notas de versão, sem materializar assets em receitas. O analisador de ações não tinha pipeline de arquivos externos. A evolução acrescenta descoberta de referências por nome, escopo verificável, metadados de artefato e execução limitada. O patch histórico pode continuar em revisão: faltam identidade e procedimento comprováveis, além de sua classe binária não ser suportada inicialmente.

## 13. Validação executada

| Verificação | Resultado |
| --- | --- |
| `cargo fmt --check` e rustfmt dos arquivos de testes incluídos | Aprovados |
| `cargo clippy --all-targets -- -D warnings` | Aprovado, zero warnings |
| `cargo test` | **326 aprovados, zero falhas, 7 ignorados** |
| Teste HTTPS controlado executado explicitamente com `--ignored` | **1 aprovado, zero falhas** |
| `npm test` | **53 aprovados em 10 arquivos**, zero falhas |
| `npm run typecheck` | Aprovado |
| `npm run build:web` | Aprovado |
| Build Tauri Linux final | **Aprovado: binário release, AppImage, DEB e RPM** |
| `git diff --check` | Aprovado |

O teste HTTPS é um dos sete ignorados na execução padrão e foi executado separadamente; os seis testes optativos anteriores de rede/ambiente real não foram acionados. O build web informa o aviso de tamanho de chunk (bundle principal acima de 600 kB); não houve erro de compilação. Houve uma falha intermediária no teste legado da sequência de eventos: o progresso por arquivo emitia também uma atualização redundante para LaunchOptions. A emissão foi limitada aos passos de arquivo, preservando a sequência original; a suíte completa foi reexecutada e passou sem remover testes.

O build final gerou `src-tauri/target/release/frx` e os três pacotes Linux sob `target/release/bundle/`, com o código final de validação dos requisitos. Esses artefatos permanecem ignorados pelo Git.

Logs, screenshots e fixtures locais ficam sob `output/playwright/file-engine/`, ignorado pelo Git.

A inspeção **nativa Tauri** usou Steam/cache/home/dados fictícios isolados e uma janela exclusiva em Xwayland/Hyprland, em **1440×900** e **1024×760**. Verificou card, detalhes de artefato, requisitos, prévia, revisão de DLL e consentimento. O botão permaneceu desabilitado com zero ou apenas uma das duas confirmações e habilitou após ambas. Nenhum botão de instalar foi acionado nessa inspeção. Arquivos da Steam fictícia permaneceram iguais; nenhuma sessão de benchmark foi criada.

Evidências locais:

- [Prévia nativa](../output/playwright/file-engine/tauri-preview-download-1440.png).
- [Consentimento em janela menor](../output/playwright/file-engine/tauri-consent-1024.png).
- [GPU não confirmada](../output/playwright/file-engine/tauri-hardware-unconfirmed-1440.png).
- [DLL em revisão](../output/playwright/file-engine/tauri-dll-review-1024.png).
- [Resultado estruturado](../output/playwright/file-engine/ui-validation.json).

O fluxo com gravação foi validado pelo Engine em instalação fictícia, incluindo transporte HTTPS real de localhost com certificados temporários, rejeição de CA desconhecida/hostname errado, download, checksum, staging, backup, instalação, histórico/reinício e rollback/reinício. O transporte de teste mapeia URLs canônicas sintéticas para o servidor TLS controlado. Não é uma instalação real de asset público no jogo do usuário. Não havia Xvfb; a janela exclusiva foi capturada sem mudar configurações globais do Hyprland e seus processos foram encerrados ao terminar.

## 14. Arquivos modificados

Inventário gerado a partir do estado Git final, abaixo. Artefatos de build, logs, certificados temporários, caches, dados da Steam fictícia e screenshots não entram nos caminhos versionáveis.

```text
README.md
docs/AUTOMATIC-FILE-FIX-ENGINE.md
docs/BATMAN-PHYSX-INVESTIGATION.md
docs/FIX-ENGINE-DINAMICO.md
docs/FIX-ENGINE.md
docs/GAME-SPECIFIC-FIX-INTELLIGENCE.md
docs/HARDWARE-FIX-INTELLIGENCE.md
docs/SMART-PACKS.md
docs/VALIDACAO-HARDWARE-FILE-ENGINE.md
src-tauri/Cargo.lock
src-tauri/Cargo.toml
src-tauri/src/commands/fix_engine.rs
src-tauri/src/commands/performance.rs
src-tauri/src/commands/smart_packs.rs
src-tauri/src/fix_engine/artifact_tests.rs
src-tauri/src/fix_engine/artifacts.rs
src-tauri/src/fix_engine/confined.rs
src-tauri/src/fix_engine/dynamic/analyzer.rs
src-tauri/src/fix_engine/dynamic/game_evidence.rs
src-tauri/src/fix_engine/dynamic/generator.rs
src-tauri/src/fix_engine/dynamic/mod.rs
src-tauri/src/fix_engine/dynamic/registry.rs
src-tauri/src/fix_engine/dynamic/tests.rs
src-tauri/src/fix_engine/file_security_tests.rs
src-tauri/src/fix_engine/https_fixture_test.rs
src-tauri/src/fix_engine/mod.rs
src-tauri/src/fix_engine/models.rs
src-tauri/src/fix_engine/planner.rs
src-tauri/src/fix_engine/security.rs
src-tauri/src/fix_engine/tests.rs
src-tauri/src/fixes/aggregation.rs
src-tauri/src/fixes/compat.rs
src-tauri/src/fixes/models.rs
src-tauri/src/fixes/providers/artifacts.rs
src-tauri/src/fixes/providers/github.rs
src-tauri/src/fixes/providers/mod.rs
src-tauri/src/fixes/providers/umu.rs
src-tauri/src/lib.rs
src-tauri/src/performance/comparison.rs
src-tauri/src/performance/mod.rs
src-tauri/src/performance/models.rs
src-tauri/src/smart_packs/benchmark.rs
src-tauri/src/smart_packs/composition.rs
src-tauri/src/smart_packs/conflicts.rs
src-tauri/src/smart_packs/discovery.rs
src-tauri/src/smart_packs/evaluation.rs
src-tauri/src/smart_packs/evidence.rs
src-tauri/src/smart_packs/lifecycle.rs
src-tauri/src/smart_packs/mod.rs
src-tauri/src/smart_packs/models.rs
src-tauri/src/smart_packs/persistence.rs
src-tauri/src/smart_packs/ranking.rs
src-tauri/src/smart_packs/tests.rs
src-tauri/src/system/context.rs
src-tauri/src/system/hardware.rs
src-tauri/src/system/mod.rs
src/components/fix/ApplyFixDialog.tsx
src/components/fix/ArtifactDetails.tsx
src/components/fix/FindingCard.tsx
src/components/fix/HardwareContext.tsx
src/components/fix/OperationHistory.tsx
src/components/fix/RecipesPanel.tsx
src/components/fix/engineMeta.ts
src/components/smart/SmartPacksPanel.tsx
src/pages/SteamGameDetail.tsx
src/services/fixEngine.ts
src/test/fileFix.test.tsx
src/types/fixEngine.ts
src/types/fixes.ts
src/types/performance.ts
src/types/smartPacks.ts
src/types/steam.ts
```

## 15. Limitações restantes

- Não há suporte automático a DLLs/PhysX ou a toda forma de patch comunitário.
- O arquivo histórico do Batman continua desconhecido; não houve medição real de FPS nesta sessão.
- GPU de execução, ICD ativo, VRAM e outros metadados podem permanecer desconhecidos; requisitos essenciais desconhecidos bloqueiam só operações dependentes deles.
- Requisitos usam correspondência exata, não intervalos inferidos ou nomes parecidos.
- O protocolo suporta instruções estreitas do publicador; não interpreta prosa arbitrária nem executa instaladores.
- Cancelamento em leitura bloqueada pode aguardar timeout; após começar a escrita, journal/recuperação conduzem a operação.
- Escritores externos não cooperantes não podem ser totalmente excluídos. Atomicidade global e segurança absoluta não são prometidas; divergências exigem recuperação.
- A instalação controlada comprova o fluxo implementado, sem comprovar disponibilidade de um patch aplicável ao Batman ou ganho de desempenho.
- Rust mínimo passou a 1.88 devido ao ZIP 9.1.1; a validação usa 1.97.

## 16. Configurações reais

Nenhum jogo real foi iniciado. Nenhum arquivo de jogo, prefixo, driver, configuração global MangoHud ou opção da Steam real foi modificado. Não houve encerramento da Steam. Benchmarks/históricos reais foram preservados. As únicas escritas de aplicação ocorreram em diretórios temporários ou fixtures identificadas no output ignorado.

## 17. Git e conclusão

No encerramento dessa validação, nenhum commit, tag, push ou release havia sido criado. Naquele momento, `git status --short` registrava **58 arquivos modificados e 14 novos**, listados acima; o índice estava vazio, `HEAD` estava em `b28d237` e a versão era 0.4.0. A inspeção dos caminhos novos/modificados não encontrou padrões de chave privada/token ou arquivos de log, backup, staging e cache em caminhos versionáveis. A implementação incremental cobre hardware contextual e operações de configuração por arquivo, com segurança e limitações documentadas. Não declara automatização de DLLs arbitrárias, identificação do patch histórico ou melhoria de FPS sem medição.
