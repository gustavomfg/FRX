# Hardware-Specific Fix Intelligence

O escopo continua sendo o Steam AppID. Hardware e software refinam esse escopo; o nome de uma variável não constitui uma recomendação universal. O conhecimento técnico permanece compartilhável, mas cada relato e medição precisa corresponder ao ambiente pertinente.

A aplicação dessas regras às configurações baixadas é descrita em [Automatic File Fix Engine](AUTOMATIC-FILE-FIX-ENGINE.md). Downloads usam o Fix Engine existente, com prévia, confirmação, backups e journal; a evolução não cria um executor paralelo nem autoriza DLLs desconhecidas.

## Inventário e GPU de execução

`system/context.rs` define `EnvironmentContext`. Ele conserva:

- GPUs detectadas: fabricante e IDs PCI, nome, endereço PCI quando verificável, módulo/versão do driver, VRAM quando disponível e identificação de chip/família do banco local;
- GPU selecionada e proveniência da seleção;
- manifestos de drivers Vulkan instalados, separados do driver Vulkan ativo;
- arquitetura da CPU, sessão gráfica, versão e origem do Proton.

AppID e build do jogo são mantidos pelo contexto do jogo e por `Conditions`, junto de CPU, sistema, kernel, resolução, preset, limite de FPS e VSync. Os campos são aditivos e opcionais, com leitura compatível com históricos anteriores.

A GPU primária do perfil é uma preferência provável para exibição, e não prova do dispositivo usado pelo jogo. `selectedGpu` só é considerado observado quando `selection == measured`. Um valor não nulo com proveniência desconhecida não seleciona silenciosamente uma GPU.

Na captura assistida, o modelo informado pelo CSV MangoHud é comparado aos dispositivos detectados. Apenas uma correspondência exata e única, com normalizações limitadas de grafia, permite associar o PCI daquele dispositivo. Duas placas com o mesmo modelo permanecem ambíguas se o log não as distinguir. Importações não recebem o perfil atual nem uma identidade PCI medida inventada.

A implementação não executa `nvidia-smi`, instala drivers, muda a GPU de execução nem altera configurações de offload.

## Fontes dos dados

- IDs e endereço PCI: `/sys/class/drm/card*/device`, incluindo `PCI_SLOT_NAME` e resolução do dispositivo físico.
- Modelo e chip/família: `pci.ids` instalado. Por exemplo, `AD107` é preservado como identificação literal de chip; não se inventa uma classificação de arquitetura comercial.
- VRAM: `mem_info_vram_total`, somente quando existe e é legível. Uso de VRAM por um processo não é confundido com capacidade total.
- Driver: módulo associado ao dispositivo e versão exposta pelo sistema, quando disponível.
- Vulkan: nomes dos manifestos ICD instalados são inventário. Eles não comprovam qual ICD o jogo carregou; `activeVulkanDriver` permanece desconhecido sem observação apropriada.
- Proton: ferramenta já resolvida pela detecção Steam, com sua versão e origem existente. Não se associa uma versão por semelhança de nome.

A detecção conserva GPUs físicas distintas mesmo quando fabricante/modelo/estado de boot são iguais. Endereços ausentes ou inválidos permanecem desconhecidos.

## Requisitos declarados pela fonte

`HardwareRequirements` contém fabricante, IDs de dispositivo, modelos, versões de driver, versões do Proton e arquitetura da CPU. São restrições exatas, sem inferência de intervalos de versão ou substituições de família.

O identificador PCI pode ser `vendor:device`, por exemplo `10de:2882`. Um ID de dispositivo isolado exige fabricante declarado, para impedir colisões entre fabricantes. Modelos diferentes de um mesmo fabricante não são considerados equivalentes.

A avaliação retorna:

- `compatible`: os requisitos declarados correspondem aos dados disponíveis;
- `unknown`: falta informação essencial para confirmar um requisito, inclusive qual GPU executa o jogo num sistema híbrido;
- `incompatible`: há informação conhecida que contradiz o requisito.

Não existir restrição de hardware não é prova de melhoria de FPS. Metadados opcionais ausentes não desabilitam todas as correções genéricas. Requisitos essenciais desconhecidos impedem automatizar uma operação por arquivo que dependa deles; a descoberta continua acessível com o motivo.

Restrições legadas de fabricante são preservadas na avaliação tipada. Em sistemas híbridos, a mera presença de uma GPU ou sua preferência como primária não confirma que ela executará o jogo.

## Artefatos e planos por hardware

A descoberta conserva separadamente a fonte da recomendação e a release/asset do publicador. Instruções estritas do publicador podem declarar `GPU-Vendor`, `GPU-Device`, `GPU-Model`, `Driver`, `Proton` e `CPU-Arch`. Esses campos alimentam os mesmos requisitos tipados usados na geração e na revalidação do plano. Uma menção a NVIDIA num comentário não confirma, sozinha, o modelo, a versão do driver ou a GPU usada pelo jogo.

Para aplicar um artefato, os requisitos essenciais declarados precisam ser compatíveis. Requisitos desconhecidos mantêm a descoberta em revisão; uma evidência favorável não ignora a allowlist, a procedência exigida ou a validação dos arquivos. Um pack misto não executa somente a parte aprovada quando outra ação ainda não pode ser validada.

A classe inicial automatizada é restrita a configurações UTF-8 INI/CFG/CONF/JSON/TXT, isoladas ou selecionadas de ZIP, com destino documentado e integridade verificável. DLLs, executáveis, scripts, TAR, XML e modificações de prefixos/Proton permanecem sem aplicação automatizada. A presença de um repositório reconhecido não certifica que todo asset seja seguro ou adequado ao hardware.

O plano inclui os arquivos afetados, origem/versão, requisitos e backups. O motor revalida a descoberta e o destino antes da aplicação e verifica os conteúdos depois do download. A atomicidade é por arquivo; journal, backups e estados de recuperação cobrem falhas de uma transação com vários arquivos. Uma atualização de jogo ou mudança externa exige nova verificação, sem reaplicação silenciosa.

## Evidências comunitárias contextuais

`Requirements.hardware` descreve a aplicabilidade da solução. Cada `PerformanceReport.hardware` também pode registrar o ambiente do relato.

O Dynamic Fix Engine filtra a evidência antes de contar relatos favoráveis ou contrários. Um relato explicitamente para outra GPU, modelo, driver ou Proton continua guardado na descoberta, mas não promove nem penaliza o ambiente atual. Fontes sem atribuição ao AppID continuam excluídas, conforme Game-Specific Fix Intelligence.

Assim, relatos positivos em RTX 4060 e negativos em uma Radeon não formam automaticamente uma contradição universal. Contradição significa evidências divergentes que se aplicam ao contexto avaliado. Relatos suficientes podem orientar um candidato; nunca substituem uma medição local ou permitem contornar regras de segurança.

## Cache e aprendizado local

O registro de receitas inclui um fingerprint do ambiente, além das entradas existentes de descoberta, catálogo, build, regras e gerador. Alterações de modelo/PCI, driver, seleção de GPU e Proton reavaliam o cache. Os arquivos e operações históricas não são descartados.

`Conditions.environment` preserva o contexto associado à captura. A comparação e o agrupamento de Smart Packs consideram diferenças conhecidas de GPU física/modelo PCI, driver, Proton/origem, arquitetura e sessão, em conjunto com as condições existentes. Inventário de outra placa não usada não substitui a GPU observada.

Quando o ambiente atual possui várias GPUs e a seleção de execução não pode ser confirmada, o FRX não apresenta o resultado de uma delas como recomendação medida para qualquer uma. Dados legados sem o contexto detalhado permanecem disponíveis com suas limitações; não recebem identificadores atuais retroativamente.

Uma atualização de driver invalida a correspondência do ambiente anterior. Uma nova versão de patch ou artefato também exige a associação de receita/fingerprint apropriada; o download ter funcionado não significa que houve ganho de desempenho.

A identidade de um Smart Pack com arquivos considera AppID, destino, SHA-256 e os identificadores/versões disponíveis do artefato, além das variáveis e receitas existentes. Uma captura de outra versão do arquivo permanece no histórico; não é promovida automaticamente a medição da configuração atual.

## Testes e limites

As fixtures de hardware são sintéticas e usadas somente dentro dos testes. A cobertura inclui:

- mesmo jogo e ação: melhoria em NVIDIA RTX 4060, regressão em AMD Radeon e impacto desconhecido em Intel Arc;
- modelos diferentes do mesmo fabricante e driver/Proton divergentes;
- placas híbridas e duas placas fisicamente distintas do mesmo modelo;
- seleção desconhecida não promovida a GPU observada;
- importação sem proveniência de GPU medida e captura enriquecida somente pelo renderer do log;
- driver/modelo/seleção alterando a chave do cache;
- ausência de metadado opcional sem bloqueio de todas as operações genéricas;
- ICD instalado não transformado em driver Vulkan ativo.

Os testes de aplicação por arquivo usam instalações e releases fictícias. Exercitam download controlado, verificação, seleção de ZIP, backups, falha parcial, retomada, divergências e rollback sem modificar a Steam real. A validação de transportes e os resultados completos ficam no documento do File Fix Engine e no relatório final; estes testes não atestam compatibilidade ou ganho de um patch binário do Batman.

Validação direcionada desta implementação:

- `cargo test system:: --lib`: **8 aprovados**, sem falhas.
- `cargo test hardware --lib`: **10 aprovados**, sem falhas (há sobreposição de testes de sysfs entre os filtros).
- Os resultados da suíte completa e do Clippy são registrados na validação final da implementação.

Os testes não são benchmarks de jogos reais. Nenhum arquivo da Steam real ou instalação real foi escrito por esses testes.

Limitações: o CSV informa um modelo de renderer, não uma atestação criptográfica do processo; duas placas idênticas podem continuar indistinguíveis. A arquitetura comercial da GPU, VRAM e ICD ativo podem permanecer desconhecidos. O reconhecimento de requisitos exatos favorece explicações verificáveis; não interpreta livremente nomes parecidos ou intervalos de driver não suportados.

Compilação: o `Cargo.toml` declara Rust mínimo **1.88**; a validação utiliza o toolchain instalado **1.97**. A extração seletiva usa `zip` **9.1.1**, sem habilitar execução de conteúdo ou extração direta para o jogo.
