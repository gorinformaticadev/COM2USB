const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const $ = (id) => document.getElementById(id);

function toast(msg) {
  const t = $("toast");
  t.textContent = msg;
  t.classList.add("show");
  clearTimeout(t._h);
  t._h = setTimeout(() => t.classList.remove("show"), 5000);
}

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
    toast(String(e));
  }
}

async function loadPorts(current) {
  const ports = await invoke("list_ports");
  const sel = $("porta");
  sel.innerHTML = "";
  for (const p of ports) {
    const o = document.createElement("option");
    o.value = p;
    o.textContent = p;
    sel.appendChild(o);
  }
  sel.value = current;
  if (sel.value !== current) {
    const o = document.createElement("option");
    o.value = current;
    o.textContent = `${current} (configurada, não detectada)`;
    sel.appendChild(o);
    sel.value = current;
  }
}

function collectSettings() {
  return {
    porta: $("porta").value,
    velocidade: Number($("velocidade").value),
    paridade: $("paridade").value,
    bitsDados: Number($("bitsDados").value),
    stopsBits: Number($("stopsBits").value),
    ativo: null,
    gravar: $("gravar").checked,
    deduplicar: $("deduplicar").checked,
  };
}

$("toggleBtn").addEventListener("click", async () => {
  const running = (await invoke("get_status")).running;
  try {
    renderStatus(await invoke("set_active", { ativar: !running }));
  } catch (e) {
    toast(String(e));
    await refreshStatus();
  }
});

$("saveBtn").addEventListener("click", async () => {
  try {
    const status = await invoke("save_settings", { settings: collectSettings() });
    renderStatus(status);
    const alOk = await invoke("set_autostart", { enabled: $("autostart").checked });
    $("autostart").checked = alOk;
    $("saveMsg").textContent = "Configurações salvas em COM2USB.INI.";
    setTimeout(() => ($("saveMsg").textContent = ""), 4000);
  } catch (e) {
    toast(String(e));
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
  $("autostart").checked = await invoke("get_autostart");

  await listen("leitura", (e) => addReading(e.payload));
  await listen("serial-error", (e) => {
    toast(String(e.payload));
    refreshStatus();
  });
}

init().catch((e) => toast(String(e)));
