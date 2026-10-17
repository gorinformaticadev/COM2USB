const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const $ = (id) => document.getElementById(id);

function alertModal(msg) {
  $("modalMsg").textContent = msg;
  $("modal").hidden = false;
  $("modalOk").focus();
}

$("modalOk").addEventListener("click", () => {
  $("modal").hidden = true;
});

function renderStatus(s) {
  const dot = $("dot");
  dot.className = "dot " + (s.error ? "err" : s.running ? "on" : "off");
  $("statusLine").textContent = s.error
    ? `Erro: ${s.error}`
    : s.running
      ? `Gravando em COM2USB.rec pela porta ${s.port}`
      : `Inativo — porta configurada: ${s.port}`;
  $("toggleBtn").textContent = s.running ? "Desativar" : "Ativar";
}

async function refreshStatus() {
  try {
    renderStatus(await invoke("get_status"));
  } catch (e) {
    alertModal(String(e));
  }
}

async function loadPorts(desejada) {
  const ports = await invoke("list_ports");
  const sel = $("porta");
  const atual = desejada ?? sel.value;
  sel.innerHTML = "";
  for (const p of ports) {
    const o = document.createElement("option");
    o.value = p;
    o.textContent = p;
    sel.appendChild(o);
  }
  if (atual && !ports.includes(atual)) {
    const o = document.createElement("option");
    o.value = atual;
    o.textContent = `${atual} (não detectada)`;
    sel.appendChild(o);
  }
  sel.value = atual || ports[0] || "";
}

window.addEventListener("focus", () => {
  loadPorts().catch(() => {});
});

function collectSettings() {
  return {
    porta: $("porta").value,
    velocidade: Number($("velocidade").value),
    paridade: $("paridade").value,
    bitsDados: Number($("bitsDados").value),
    stopsBits: Number($("stopsBits").value),
    ativo: false,
    gravar: $("gravar").checked,
    deduplicar: $("deduplicar").checked,
    digitar: $("digitar").checked,
    enter: $("enter").checked,
  };
}

$("toggleBtn").addEventListener("click", async () => {
  const running = (await invoke("get_status")).running;
  try {
    renderStatus(await invoke("set_active", { ativar: !running }));
  } catch (e) {
    alertModal(String(e));
    await refreshStatus();
  }
});

let autostartSalvo = false;

$("saveBtn").addEventListener("click", async () => {
  try {
    renderStatus(await invoke("save_settings", { settings: collectSettings() }));
    $("saveMsg").textContent = "Configurações salvas em COM2USB.INI.";
    setTimeout(() => ($("saveMsg").textContent = ""), 4000);
  } catch (e) {
    alertModal(String(e));
  }
  if ($("autostart").checked === autostartSalvo) return;
  try {
    autostartSalvo = await invoke("set_autostart", { enabled: $("autostart").checked });
  } catch (e) {
    $("autostart").checked = autostartSalvo;
    alertModal(String(e));
  }
});

$("wedgeBtn").addEventListener("click", async () => {
  const msg = $("saveMsg");
  for (const s of ["3", "2", "1"]) {
    msg.textContent = `Clique no documento de destino. Digitará em ${s}…`;
    await new Promise((r) => setTimeout(r, 1000));
  }
  msg.textContent = "";
  try {
    await invoke("wedge_test", { texto: "1234567890" });
  } catch (e) {
    alertModal(String(e));
  }
});

function addReading({ codigo, dataHora, duplicado }) {
  const tbody = $("readings");
  tbody.querySelector("tr.empty")?.remove();
  const tr = document.createElement("tr");
  const td1 = document.createElement("td");
  td1.textContent = dataHora;
  const td2 = document.createElement("td");
  td2.textContent = codigo;
  const td3 = document.createElement("td");
  if (duplicado) {
    const b = document.createElement("span");
    b.className = "dup";
    b.textContent = "DUP";
    td3.appendChild(b);
  }
  tr.append(td1, td2, td3);
  tbody.prepend(tr);
  while (tbody.children.length > 100) tbody.lastChild.remove();
}

async function init() {
  const [s, cfg] = await Promise.all([invoke("get_status"), invoke("get_settings")]);
  renderStatus(s);
  await loadPorts(cfg.porta);
  $("velocidade").value = String(cfg.velocidade);
  $("paridade").value = cfg.paridade;
  $("bitsDados").value = String(cfg.bitsDados);
  $("stopsBits").value = String(cfg.stopsBits);
  $("gravar").checked = Boolean(cfg.gravar);
  $("deduplicar").checked = Boolean(cfg.deduplicar);
  $("digitar").checked = Boolean(cfg.digitar);
  $("enter").checked = Boolean(cfg.enter);
  autostartSalvo = await invoke("get_autostart");
  $("autostart").checked = autostartSalvo;

  await listen("leitura", (e) => addReading(e.payload));
  await listen("serial-error", (e) => {
    alertModal(String(e.payload));
    refreshStatus();
  });
}

init().catch((e) => alertModal(String(e)));
