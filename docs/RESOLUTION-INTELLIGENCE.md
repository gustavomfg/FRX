# FRX Resolution Intelligence — desenvolvimento da v0.5

A Resolution Intelligence integra diagnóstico, fontes e planos por jogo à arquitetura existente. Ela não substitui o Fix Engine, não cria outro executor e não trata desempenho como finalidade de toda correção. Um plano pode investigar crash, compatibilidade, qualidade gráfica, física ou estabilidade sem prometer ganho de FPS.

O suporte entregue é incremental. Existem diagnósticos conservadores, pesquisa pública com proveniência, planos aplicáveis pelas operações já suportadas e planos explicitamente bloqueados para revisão. Não há correção automática universal de PhysX, DLSS, FSR, ray tracing ou frame generation. Os resultados de validação consolidados estão em [VALIDACAO-v0.5.md](VALIDACAO-v0.5.md); fontes e requisitos gráficos estão em [GRAPHICS-PHYSICS-DIAGNOSTICS.md](GRAPHICS-PHYSICS-DIAGNOSTICS.md).

## Arquitetura reutilizada

| Camada existente | Papel no fluxo |
| --- | --- |
| Steam Game Discovery | AppID, instalação real, biblioteca e ferramenta de compatibilidade |
| Hardware Intelligence | Inventário de GPUs, GPU observada quando disponível, driver e contexto Proton |
| Fix Discovery | Descobertas GitHub/UMU, atribuição por jogo, cache e proveniência |
| Dynamic Fix Engine | Receitas tipadas, allowlist de operações e revalidação de aplicabilidade |
| File Fix Engine | Preparação, confiança de artefatos, download autorizado, staging e operações de arquivo suportadas |
| Fix Engine transacional | Prévia, confirmação, backup, journal, aplicação, recuperação e rollback |
| Smart Packs | Composição, conflitos, ranking contextual e comparação com baseline |
| Performance Lab | Sessões reais, condições de teste, séries e métricas; fixtures continuam identificadas como sintéticas |

Os novos módulos em `src-tauri/src/resolution/` separam modelos, inspeção local (`detection`), pesquisa (`research`), planos documentados em revisão (`plans`), persistência e orquestração. `commands/resolution.rs` expõe cinco comandos Tauri. No front-end, `types/resolution.ts` espelha os modelos, `services/resolution.ts` encapsula IPC e `ResolutionCenter.tsx` integra a página de um jogo Steam usando o diálogo de prévia já existente.

## Modelo de conhecimento

Um `Report` pertence a um AppID e conserva ambiente, fingerprint do contexto, sintomas, tecnologias, problemas, planos, referências de pacotes, conhecimento consultado e limitações. Cada `Evidence` conserva tipo, resumo, URL/publicador quando disponíveis, datas separadas, versão, AppID opcional, tecnologia, estado de cache e limitações.

Há quatro origens distintas:

1. **Conhecimento técnico geral:** documentação e releases de projetos. `appId` fica ausente; esse conteúdo não prova que o jogo implementa um recurso.
2. **Relato específico do jogo:** fonte com atribuição verificável ao AppID. Não equivale a reprodução local nem garante compatibilidade com outro hardware.
3. **Observação local:** inspeção estática e padrões reconhecidos de logs importados pelo usuário, conservando suas limitações.
4. **Medição local:** sessões do Performance Lab e regras existentes de comparabilidade. A camada de pesquisa não inventa benchmarks.

Descoberta, diagnóstico e execução permanecem separados. Texto externo nunca adquire autoridade para executar shell, ampliar allowlists, selecionar destinos arbitrários ou instalar DLLs. A fonte de uma solução pode justificar investigação; a aplicação depende de uma receita validada pelo Fix Engine.

## Diagnóstico e capacidades

Cada tecnologia possui estados independentes para hardware, implementação do jogo, ambiente Proton/driver, configuração e execução. Os estados desconhecidos são preservados. Encontrar `nvngx_dlss.dll`, por exemplo, produz indício estático, não “DLSS ativo”. Um banner DXVK/VKD3D identifica um componente em um log, não comprova apresentação correta ou ausência de falhas.

A inspeção percorre a instalação resolvida pelo Game Discovery, com limites de profundidade, quantidade de entradas e leitura. Não segue symlinks para sair da instalação. Dados de configuração aceitos são analisados como texto, sem execução. Ausência em uma inspeção limitada não é tratada como prova de que o recurso inexiste.

A regra de hardware NVIDIA tem fonte e data de revisão explícitas. Distingue DLSS comum, Frame Generation e Multi Frame Generation. O jogo e o ambiente continuam exigindo evidência independente. FSR não é presumido exclusivo de AMD; DX12 não implica ray tracing; uma GPU NVIDIA não comprova PhysX por GPU no Proton.

Os resolvers produzem evidências observadas, hipóteses, requisitos, limitações e próxima ação para física, upscaling, geração de quadros, ray tracing, pilha gráfica, estabilidade, áudio, vídeo e controles. Alguns subsistemas permanecem em diagnóstico ou orientação quando não há sinal verificável ou operação suportada. O motor pode concluir “Problema identificado — nenhuma solução aplicável confirmada”.

## Logs autorizados

A importação é iniciada pelo usuário por um seletor de arquivo. Não há coleta automática de logs pessoais. O parser aceita texto UTF-8 regular de até 2 MiB, rejeita links e arquivos especiais e confere alterações durante a leitura. O cabeçalho precisa identificar exclusivamente o AppID selecionado por `SteamGameId:` ou `SteamAppId:`; nome do arquivo e consulta não comprovam escopo.

Somente padrões reconhecidos são convertidos em evidências. O log bruto, caminhos pessoais e comandos contidos nele não são armazenados no relatório. Ficam digest, data de importação, observações resumidas e limitações. Um log importado não autentica o processo que o produziu, não prova a data da execução e não se torna benchmark. Logs DXVK/VKD3D sem identificação confiável do jogo podem ser recusados; importar o log Proton correspondente é o caminho suportado.

Componentes observados e falhas podem coexistir e formar evidência conflitante. Ausência de mensagem de erro não é interpretada como resolução de crash. A média de FPS do MangoHud não verifica geração de quadros ou latência.

## Pesquisa pública e cache

`research_game_resolutions` atualiza primeiro as descobertas pelos provedores existentes e consulta os adaptadores de documentação/release/issue. A seleção usa sintoma, AppID e ferramenta Proton. O fabricante da GPU só restringe a pesquisa quando a GPU utilizada foi observada. Não se envia o conteúdo de logs importados aos provedores.

O Research Engine usa o cliente HTTP restrito existente: HTTPS, validação TLS, lista de hosts, controle de redirecionamentos, timeout, limites de bytes, ETag e rate limits. Consulta até seis documentos, uma release e uma busca com até cinco issues. Documentos têm teto de 256 KiB; respostas da API, 512 KiB. Não baixa assets durante a pesquisa.

Fontes gerais podem compartilhar cache HTTP por URL. Resultados específicos mantêm seu AppID e não são reatribuídos ao trocar de jogo. As consultas incluem AppID, mas apenas o conteúdo da fonte confirma a identidade; títulos com identificação conflitante são recusados. Cache vencido continua disponível em falhas de rede, marcado como tal. JSON malformado, fonte ausente e limite de consultas produzem limitação explícita, sem simular resultados.

A identidade da evidência incorpora o digest do conteúdo e da URL. Releases incluem os metadados na identidade, porque versão, datas e assets podem mudar sem alterar a prosa. As datas de publicação, atualização e consulta não são intercambiáveis. Um hash identifica conteúdo; não demonstra que um artefato é benigno.

## Comandos Tauri

| Comando | Efeito |
| --- | --- |
| `get_resolution_report(appId)` | Carrega um relatório existente e reconfere o estado atual antes de devolvê-lo; não pesquisa a rede |
| `diagnose_game(appId, symptoms)` | Reúne contexto e observações disponíveis, gera relatório e persiste dados próprios do FRX |
| `research_game_resolutions(appId, symptoms)` | Consulta as fontes permitidas, atualiza conhecimento e refaz o diagnóstico |
| `import_resolution_log(appId, path)` | Analisa somente o arquivo escolhido e atribuído ao jogo, conserva observações limitadas e refaz o diagnóstico |
| `verify_resolution_plan(appId, planId)` | Reconfere a operação registrada no Fix Engine e seus efeitos no disco; não inicia o jogo |

Os comandos validam AppID e sintomas e serializam alterações no armazenamento da Resolution Intelligence. Não existe comando novo que execute diretamente texto de pesquisa. Aplicação e rollback continuam usando os comandos existentes do Fix Engine, com a prévia e a confirmação exigidas por ele.

## Planos aplicáveis e planos em revisão

Descobertas específicas geram planos vinculados às receitas existentes. O plano inclui finalidade, evidências, requisitos, ações, riscos, reversibilidade, método de verificação e critério de sucesso. A receita é revalidada; um estado informativo “aplicável” não substitui a prévia nem sua revalidação antes da escrita.

O fluxo vertical de diagnóstico de inicialização/crash reutiliza a receita revisada `frx.diagnostics.proton-log`: após confirmação, define `PROTON_LOG=1` preservando as demais opções. A operação prepara uma observação posterior; não afirma corrigir o crash. A execução do jogo continua sendo decisão e ação do usuário. A importação do log e a verificação do estado persistido completam a investigação possível sem fabricar resultado runtime.

Planos documentados de GE-Proton para `PROTON_DLSS_UPGRADE` e `PROTON_FSR4_UPGRADE` ficam **Requer revisão**, sem `recipeId` executável. A documentação da tag exata é distinguida de master e de outras variantes de Proton. Mesmo quando a versão corresponde, atualização de binário por manifesto/fallback pode ter efeitos que retirar uma variável não desfaz. Faltam política específica de artefato fixado, instalação, backup e rollback para promover esse mecanismo à aplicação automática.

Troca de Proton, instaladores arbitrários, binários desconhecidos e alterações de driver também não recebem execução automática por esta camada. As classes de configuração suportadas continuam disponíveis pelo File Fix Engine, com suas políticas anteriores. Nenhuma categoria gráfica recebe um botão de aplicação sem operação real correspondente.

## Smart Resolution Packs e desempenho

A composição reutiliza o Smart Packs Builder. Só associa planos do mesmo AppID e não agrupa problemas diferentes apenas para preencher categorias. Os pacotes conservam a referência ao Smart Pack real e o propósito identificado; não há pacotes fictícios de PhysX ou DLSS para preencher a interface.

O Performance Lab continua responsável por capturas, baseline, comparação e evidência contextual. Um link para o Lab abre o jogo correspondente; não inicia um teste ou uma aplicação silenciosa. O fluxo já existente de “Testar desempenho” mantém associação ao pack e avaliação automática quando iniciado pelo usuário no Smart Packs.

Nenhum resultado de desempenho é criado por copiar arquivo, ativar variável ou encontrar um relato externo. Uma melhoria de qualidade pode reduzir FPS e deve ser verificada com o método pertinente. Resultados de benchmark não confirmam automaticamente qualidade visual, física, geração de quadros ou latência.

## Persistência, revalidação e rollback

Inputs e relatórios são armazenados no diretório próprio `resolution` do Fix Engine, por AppID, com schema explícito, arquivos privados e limite de 4 MiB. Os ancestrais são abertos por descritores sem seguir symlinks; o diretório permanece fixado durante leitura e escrita. A escrita atômica existente é reutilizada, com diretórios 0700, arquivos 0600 e recusa de hardlinks no destino. Arquivos malformados, de outro jogo ou formato incompatível são preservados e geram erro, sem migração silenciosa para o jogo aberto. As evidências aninhadas também mantêm escopo.

Ao reabrir um relatório, o FRX reconstrói o contexto e confere o estado real das operações. “Configuração conferida” só é preservado para o mesmo contexto e operação, após nova conferência. O fingerprint da receita aplicada é comparado com o da receita atual: mudança de fonte ou receita não transforma uma aplicação antiga em confirmação do plano novo.

O journal e os backups continuam sendo a autoridade para recuperação. A indisponibilidade atual de uma receita não deve apagar a operação antiga nem impedir a revisão do rollback. O usuário abre a prévia de desfazer e confirma; o Fix Engine detecta divergências e não sobrescreve silenciosamente alterações externas. A Resolution Intelligence não implementa uma segunda rotina de restauração.

## Estados de verificação

| Estado | Significado |
| --- | --- |
| `applicable` | Há receita suportada; exige prévia e confirmação |
| `review` | Plano incompleto, incompatível ou sem política de execução/reversão suficiente |
| `applied-unverified` | O Fix Engine aplicou a operação; funcionamento do recurso/sintoma ainda não foi verificado |
| `verified-configuration` | O efeito persistido foi conferido no disco; o problema do jogo continua exigindo evidência independente |
| `rolled-back` | Operação restaurada pelo Fix Engine; isso não inventa resultado no jogo |
| `error` | Divergência, falha, alteração de fonte ou estado que exige atenção |

Nenhum desses estados declara “problema resolvido” apenas pela escrita. Verificação de configuração é uma capacidade real e limitada. Verificação automática de DLSS/PhysX/FG em execução permanece indisponível quando não existe instrumentação confiável.

## Interface e uso

Na página real de um jogo Steam, o Resolution Center permite selecionar sintomas, diagnosticar, pesquisar e importar log. As tecnologias aparecem com seus estados independentes. Os detalhes dos problemas mostram evidência, hipóteses, fontes, requisitos, riscos, plano e verificação. A aplicação usa `ApplyFixDialog`; desfazer consulta a operação pertencente ao mesmo AppID. Trocar de jogo descarta respostas e diálogos pendentes do jogo anterior.

O modo demonstrativo não consulta fontes nem aplica essas correções. Fontes são abertas apenas pelos esquemas/domínios permitidos e conteúdo externo é renderizado como texto. A identidade visual e os personagens existentes são reutilizados.

## Limitações atuais

- A inspeção estática reconhece componentes e configurações limitados, sem análise universal de executáveis ou APIs.
- Fontes públicas podem não ter AppID, requisitos suficientes ou instruções instaláveis. Nesses casos, não nasce receita automática.
- Diagnóstico de áudio, vídeo e controles depende de sintomas/logs reconhecidos; não há verificação universal desses subsistemas.
- Informações opcionais de driver, Proton e GPU podem permanecer desconhecidas.
- O modelo não autentica logs enviados pelo usuário; as limitações acompanham as observações e eles não alimentam ranking de FPS.
- Não há correção automática comprovada do patch PhysX histórico do Batman. O arquivo original continua não identificado.
- Atualizações de componentes proprietários, troca automática de Proton e recursos avançados sem política específica permanecem em revisão.

Os testes usam instalações, logs e fontes controlados explicitamente identificados como fixtures. Eles validam a arquitetura e os limites, sem substituir medições em jogo real. Consulte [VALIDACAO-v0.5.md](VALIDACAO-v0.5.md) para resultados, ambiente nativo, builds e pendências.
