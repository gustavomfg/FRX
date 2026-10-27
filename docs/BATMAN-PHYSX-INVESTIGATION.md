# Investigação: Batman Arkham City GOTY / NVIDIA PhysX

Consulta pública realizada em 27/10/2026, sem iniciar o jogo, ler arquivos da instalação real ou baixar/executar binários. Steam AppID de referência: `200260`. A identidade do arquivo que o usuário instalou historicamente **continua não confirmada**. Os valores informais mencionados pelo usuário não são benchmarks registrados nem foram usados como evidência local.

## Fontes primárias encontradas

### Relato de Arkham City que aponta para nvidia-libs

A [issue #196 do GE-Proton](https://github.com/GloriousEggroll/proton-ge-custom/issues/196), aberta em 07/08/2025, relata ausência de aceleração GPU de PhysX em Batman Arkham City e aponta para `SveSop/nvidia-libs`. Trata-se de relato comunitário, com medições declaradas pelo autor fora do FRX. O título não contém AppID, e o texto consultado não confirma edição GOTY, versão do jogo, GPU e instalação suficientes para reproduzir a configuração do usuário. É uma pista pertinente, sem atribuição automática de benchmark ou confirmação do patch histórico.

### Projeto e artefatos publicados

O [repositório do mantenedor SveSop/nvidia-libs](https://github.com/SveSop/nvidia-libs) reúne implementações de bibliotecas NVIDIA para Wine/Proton. A documentação descreve instalação em prefixos ou na própria ferramenta Proton, usando procedimentos externos, além de opções para ativação das bibliotecas. Isso difere de uma instrução confirmada para copiar um arquivo específico para a instalação de Arkham City. O FRX não executou esses procedimentos.

Os metadados públicos da [release v1.0.3](https://github.com/SveSop/nvidia-libs/releases/tag/v1.0.3), publicados em 23/10/2026, identificaram dois assets `.tar.xz` e digests SHA-256. O arquivo `nvidia-libs-v1.0.3.tar.xz` tem tamanho publicado de 2.570.372 bytes e digest publicado `7557c9ef7dc26d1a7d5b561acf5d756319cec05cf30b9cb756efd81a7f472feb`. O payload não foi baixado; não foi feita verificação independente desses bytes. A assinatura não foi confirmada. Uma release recente não identifica o arquivo instalado historicamente pelo usuário.

Fonte dos metadados: [API da release 421827008](https://api.github.com/repos/SveSop/nvidia-libs/releases/421827008). A consulta inicial por versão identifica esse ID; o FRX conserva e revalida o endpoint canônico por ID antes da instalação, em vez de confiar que `latest` ou uma tag permaneçam iguais.

O subprojeto [SveSop/nvcuda](https://github.com/SveSop/nvcuda) também publica bibliotecas, mas não foi encontrada instrução suficiente para atribuir uma substituição isolada de DLL ao AppID 200260 e à configuração do usuário. Origem identificada e digest publicado são elementos de procedência/integridade; não atestam que um binário seja benigno, compatível ou mais rápido.

### Compatibilidade atual do Proton CachyOS

O [README do Proton CachyOS](https://github.com/CachyOS/proton-cachyos/blob/cachyos_main/README.md) documenta opções para bibliotecas NVIDIA e PhysX, incluindo dependências e limitações com WoW64. A mesma documentação contém ressalva específica sobre bibliotecas de 32 bits em GPUs RTX série 4000/5000. Isso impede tratar “NVIDIA” como requisito suficiente e reforça a necessidade de identificar versão do Proton, arquitetura do jogo/biblioteca e driver. Não foi confirmada a versão utilizada na instalação histórica.

### Evitar confundir Arkham Asylum com Arkham City

O [README do BmLauncher](https://github.com/neatodev/BmLauncher#hint-for-linux-users) menciona copiar `PhysXDevice.dll` de Arkham City para **Arkham Asylum**, cujo AppID é `35140`. Essa instrução não é uma correção para instalar em Arkham City (`200260`). O nome da DLL e a presença de “Arkham City” na frase não autorizam transportar a receita para o jogo errado.

A [issue #309 do Proton](https://github.com/ValveSoftware/Proton/issues/309) fornece o vínculo direto com Arkham City GOTY / 200260 e registra vários problemas de compatibilidade ao longo do tempo; uma solução ali citada precisa conservar data, contexto e limitações próprios.

## Por que a pesquisa anterior não cobria essa solução

| Pergunta | Situação anterior e mudança implementada |
| --- | --- |
| Ignorava Releases e anexos? | Lia notas de versão do Proton/GE, mas não materializava seus assets em candidatos de arquivo. Agora lê metadados de Releases referenciadas por fontes integradas e mantém anexos/downloads ambíguos como revisão, sem baixar o payload. |
| Reconhecia substituição de arquivos? | A análise estática sinalizava downloads ou passos de arquivo como não suportados; não produzia um plano geral de instalação de artefatos. Agora existe modelo tipado com origem, digest e mapeamentos documentados. |
| Separava recomendação da origem do download? | Uma URL no comentário não era um artefato validado. O novo modelo mantém a fonte da recomendação separada do repositório, release, publicador, asset e metadados de integridade. |
| Encontrava instruções sem AppID no título? | A consulta por `AppID in:title` perdia relatos como GE #196. Há uma consulta adicional limitada por nome nos trackers integrados; ela só descobre referências. Nome/consulta não provam AppID. O vínculo pode ser confirmado pelo identificador da própria issue ou pela documentação explícita do publicador. |
| Conseguia determinar o destino? | Não havia interpretação geral segura. Agora entende instruções estritas do publicador para destinos relativos e membros selecionados, ou manifesto tipado. Prosa vaga não gera destino. |
| Reconhecia NVIDIA? | Já existia fabricante; agora os requisitos explícitos do publicador também preservam modelo, PCI, driver, Proton e arquitetura. Uma placa detectada não é prova de que o jogo a utiliza. |
| Uma solução válida vira receita? | Sim, para a classe inicial de arquivos de configuração/texto com publicador revisado, digest publicado, vínculo e destinos verificáveis; o núcleo revalida a política e o conteúdo antes da escrita. Binários/DLLs e procedimentos externos permanecem em revisão. |
| Explica quando não aplica? | O candidato conserva motivos específicos: AppID não confirmado, origem não revisada, digest ausente, destino desconhecido, formato/binário não suportado, requisitos incompatíveis ou etapa externa necessária. |

## Resultado desta investigação

Foi encontrada uma pista verificável para a família de soluções NVIDIA/PhysX, com projeto e releases reais. **Não foi identificado o arquivo histórico exato**, nem um destino de instalação de DLL suficientemente documentado para Arkham City GOTY, RTX 4060 e a versão do Proton do usuário.

Nenhum “Batman NVIDIA Performance Pack” foi criado fingindo que esse patch já estava validado. `nvidia-libs` envolve binários, pacote TAR e procedimentos para prefixo/Proton; essa classe permanece **Requer revisão** na implementação inicial. O caso não foi convertido em uma instrução simplificada de copiar DLL, nem em promessa de FPS.

## Contrato de descoberta de arquivos suportado

O FRX consulta APIs públicas do GitHub a partir de referências em issues/comentários verificados, arquivos UMU lidos estaticamente e releases integradas. O limite inicial é de quatro referências por consulta e oito assets por release; os metadados ficam no cache HTTP existente. Não há execução de Python, shell ou scripts de instalação.

O publicador pode descrever uma operação simples diretamente nas notas da release, por exemplo:

```text
Steam AppID: 123456
Platform: windows
Architecture: any
Install: Config/fix.ini <- fix.ini
GPU-Vendor: NVIDIA
GPU-Device: 10de:2882
GPU-Model: GeForce RTX 4060
Driver: versão-exata-publicada
Proton: versão-exata-publicada
CPU-Arch: x86_64
License: licença-informada-pelo-publicador
```

Esses dados são um **exemplo de protocolo**, não uma correção real ou cadastro individual do Batman. `Replace:` indica substituição explícita. Para ZIP, o formato é `Extract: pacote.zip::caminho/membro.ini -> Config/membro.ini`, com `SHA256 (caminho/membro.ini)=<digest>` publicado por membro. Um bloco `frx-file-fix` com formato tipado pode declarar a mesma operação. O hash do arquivo baixado precisa também constar nos metadados oficiais do asset.

A convenção reutilizável permite novas correções publicadas pelas fontes sem editar JSON por jogo no código do FRX. Não autoriza o publicador a escapar do diretório do jogo ou contornar a política. Instruções incompletas, contraditórias ou que exigem comandos/registro/dependências desconhecidas não viram aplicação parcial. Prosa não reconhecida fora de um manifesto e procedimentos que dependam de múltiplos assets ficam em revisão nesta primeira versão; apenas vários membros explicitamente selecionados de um mesmo ZIP podem formar uma operação conjunta.

Os publicadores inicialmente reconhecidos são ValveSoftware/Proton, GloriousEggroll/proton-ge-custom, Open-Wine-Components/umu-protonfixes, CachyOS/proton-cachyos, doitsujin/dxvk, HansKristian-Work/vkd3d-proton, jp7677/dxvk-nvapi, SveSop/nvidia-libs e SveSop/nvcuda. Essa lista é uma política técnica de procedência revisável, não uma certificação de segurança ou desempenho. A execução inicial limita-se a dados de configuração UTF-8 INI/CFG/CONF/JSON/TXT com validação adicional de conteúdo; XML, scripts, DLLs, executáveis, TAR e procedimentos que alterem prefixos/Proton exigem revisão.

## Verificação da descoberta

Em 27/10/2026, `cargo test --manifest-path src-tauri/Cargo.toml fixes:: --lib` terminou com **62 testes aprovados, zero falhas e um ignorado** (consulta real de rede optativa). Essa suíte cobre identidade do jogo, referências e metadados de Releases sem baixar payload, comentários sem autoridade de destino, AppID estrangeiro, requisito de hardware inválido, checksum contraditório, DLL sem suporte, traversal e rejeição de procedimentos parciais. São testes isolados com fixtures explícitas, não validação de desempenho ou instalação de um patch real de PhysX.
