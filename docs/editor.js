import module_loader from './gg3z.js';

const Module = await module_loader({});
window.Module = Module;

let parse_module = Module.cwrap('parse_module', 'number', ['string']);
window.parse_module = parse_module;

let module_to_string = Module.cwrap('module_to_string', 'string', ['number']);
window.module_to_string = module_to_string;

let free_module = Module.cwrap('free_module', null, ['number']);
window.free_module = free_module;

let module_stage_count = Module.cwrap('module_stage_count', 'number', ['number']);
let module_get_stage_name = Module.cwrap('module_get_stage_name', 'string', ['number', 'number']);

let create_frontend = Module.cwrap('create_frontend', 'number', []);
let free_frontend = Module.cwrap('free_frontend', null, ['number']);
let frontend_load = Module.cwrap('frontend_load', 'number', ['number', 'string']);
let frontend_query = Module.cwrap('frontend_query', 'string', ['number', 'string']);
let frontend_fact_count = Module.cwrap('frontend_fact_count', 'number', ['number']);
let frontend_rule_count = Module.cwrap('frontend_rule_count', 'number', ['number']);
let frontend_stage_count = Module.cwrap('frontend_stage_count', 'number', ['number']);
let frontend_stage_name = Module.cwrap('frontend_stage_name', 'string', ['number', 'number']);
let frontend_set_strategy = Module.cwrap('frontend_set_strategy', null, ['number', 'number']);
let frontend_get_strategy = Module.cwrap('frontend_get_strategy', 'number', ['number']);
let frontend_set_max_steps = Module.cwrap('frontend_set_max_steps', null, ['number', 'number']);
let frontend_get_max_steps = Module.cwrap('frontend_get_max_steps', 'number', ['number']);
let frontend_query_start = Module.cwrap('frontend_query_start', 'string', ['number', 'string', 'number']);
let frontend_query_next = Module.cwrap('frontend_query_next', 'string', ['number']);
let frontend_has_more = Module.cwrap('frontend_has_more', 'number', ['number']);
let frontend_query_reason = Module.cwrap('frontend_query_reason', 'number', ['number']);
let frontend_query_stop = Module.cwrap('frontend_query_stop', null, ['number']);
let frontend_run_stage = Module.cwrap('frontend_run_stage', 'number', ['number', 'number']);
let frontend_state_var_count = Module.cwrap('frontend_state_var_count', 'number', ['number']);
let frontend_state_var_name = Module.cwrap('frontend_state_var_name', 'string', ['number', 'number']);
let frontend_state_var_value = Module.cwrap('frontend_state_var_value', 'string', ['number', 'number']);
let frontend_query_batch = Module.cwrap('frontend_query_batch', 'string', ['number', 'string', 'number', 'number']);
let frontend_add_fact = Module.cwrap('frontend_add_fact', 'number', ['number', 'string']);
let frontend_clear_facts_by_relation = Module.cwrap('frontend_clear_facts_by_relation', null, ['number', 'string']);
let frontend_collect_draws = Module.cwrap('frontend_collect_draws', 'number', ['number', 'number']);
let frontend_collect_draws_by_name = Module.cwrap('frontend_collect_draws_by_name', 'number', ['number', 'string']);
let frontend_draw_command_name = Module.cwrap('frontend_draw_command_name', 'string', ['number', 'number']);
let frontend_draw_command_arg_count = Module.cwrap('frontend_draw_command_arg_count', 'number', ['number', 'number']);
let frontend_draw_command_arg = Module.cwrap('frontend_draw_command_arg', 'number', ['number', 'number', 'number']);

const reloadBtn = document.querySelector('#reload');
const stagesSpan = document.querySelector('#stages');
const stateVarsDiv = document.querySelector('#state-vars');
const inp = document.querySelector('#input');
const out = document.querySelector('#output');
const queryInput = document.querySelector('#query-input');
const queryBtn = document.querySelector('#query-btn');
const nextBtn = document.querySelector('#next-btn');
const stopBtn = document.querySelector('#stop-btn');
const strategySelect = document.querySelector('#strategy-select');
const maxStepsInput = document.querySelector('#max-steps-input');
const stepBtn = document.querySelector('#step-btn');
const runBtn = document.querySelector('#run-btn');
const pauseBtn = document.querySelector('#pause-btn');
const canvasContainer = document.querySelector('#canvas-container');
const keysList = document.querySelector('#keys-list');

let currentModule = null;
let frontend = create_frontend();
let queryStartTime = null;
let lastRunStage = -1;
let drawStageIndex = -1;
let isRunning = false;
let animationId = null;
let lastFrameTime = 0;
const FRAME_INTERVAL = 1000 / 30; // 30 FPS

const CANVAS_WIDTH = 800;
const CANVAS_HEIGHT = 400;
const SCALE = 8;

const app = new PIXI.Application({
    width: CANVAS_WIDTH,
    height: CANVAS_HEIGHT,
    backgroundColor: 0x1a1a2e,
});
canvasContainer.appendChild(app.view);

const graphics = new PIXI.Graphics();
app.stage.addChild(graphics);

const pressedKeys = new Set();

function updateKeysDisplay() {
    if (pressedKeys.size === 0) {
        keysList.textContent = '(none)';
    } else {
        keysList.textContent = Array.from(pressedKeys).join(', ');
    }
}

function keyToAtom(key) {
    const keyMap = {
        ' ': 'space',
        'ArrowUp': 'up',
        'ArrowDown': 'down',
        'ArrowLeft': 'left',
        'ArrowRight': 'right',
        'Enter': 'enter',
        'Escape': 'escape',
        'Shift': 'shift',
        'Control': 'ctrl',
        'Alt': 'alt',
    };
    return keyMap[key] || key.toLowerCase();
}

document.addEventListener('keydown', (e) => {
    if (e.target === inp) return;
    const atom = keyToAtom(e.key);
    if (!pressedKeys.has(atom)) {
        pressedKeys.add(atom);
        frontend_add_fact(frontend, `key_pressed(${atom})`);
        updateKeysDisplay();
    }
    if (e.key === ' ' || e.key.startsWith('Arrow')) {
        e.preventDefault();
    }
});

document.addEventListener('keyup', (e) => {
    if (e.target === inp) return;
    const atom = keyToAtom(e.key);
    pressedKeys.delete(atom);
    frontend_clear_facts_by_relation(frontend, 'key_pressed');
    for (const k of pressedKeys) {
        frontend_add_fact(frontend, `key_pressed(${k})`);
    }
    updateKeysDisplay();
});

function findStageIndices() {
    const count = frontend_stage_count(frontend);
    drawStageIndex = -1;
    for (let i = 0; i < count; i++) {
        const name = frontend_stage_name(frontend, i);
        if (name === 'Draw') drawStageIndex = i;
    }
}

function drawRects() {
    graphics.clear();

    graphics.lineStyle(1, 0x444466);
    for (let x = 0; x <= CANVAS_WIDTH; x += SCALE) {
        graphics.moveTo(x, 0);
        graphics.lineTo(x, CANVAS_HEIGHT);
    }
    for (let y = 0; y <= CANVAS_HEIGHT; y += SCALE) {
        graphics.moveTo(0, CANVAS_HEIGHT - y);
        graphics.lineTo(CANVAS_WIDTH, CANVAS_HEIGHT - y);
    }

    if (drawStageIndex < 0) return;

    const count = frontend_collect_draws(frontend, drawStageIndex);
    if (count < 0) return;

    graphics.beginFill(0x00ff88);
    for (let i = 0; i < count; i++) {
        const name = frontend_draw_command_name(frontend, i);
        if (name === 'rect') {
            const argCount = frontend_draw_command_arg_count(frontend, i);
            if (argCount >= 4) {
                const x = frontend_draw_command_arg(frontend, i, 0);
                const y = frontend_draw_command_arg(frontend, i, 1);
                const w = frontend_draw_command_arg(frontend, i, 2);
                const h = frontend_draw_command_arg(frontend, i, 3);
                const screenX = x * SCALE;
                const screenY = CANVAS_HEIGHT - (y + h) * SCALE;
                const screenW = w * SCALE;
                const screenH = h * SCALE;
                graphics.drawRect(screenX, screenY, screenW, screenH);
            }
        }
    }
    graphics.endFill();
}

function step() {
    const count = frontend_stage_count(frontend);
    for (let i = 0; i < count; i++) {
        if (i === drawStageIndex) break;
        frontend_run_stage(frontend, i);
    }
    drawRects();
    updateStateVars();
}

stepBtn.onclick = step;

function gameLoop(timestamp) {
    if (!isRunning) return;
    const elapsed = timestamp - lastFrameTime;
    if (elapsed >= FRAME_INTERVAL) {
        lastFrameTime = timestamp - (elapsed % FRAME_INTERVAL);
        step();
    }
    animationId = requestAnimationFrame(gameLoop);
}

runBtn.onclick = () => {
    reloadBtn.click();
    isRunning = true;
    runBtn.disabled = true;
    pauseBtn.disabled = false;
    gameLoop();
};

pauseBtn.onclick = () => {
    isRunning = false;
    if (animationId) {
        cancelAnimationFrame(animationId);
        animationId = null;
    }
    runBtn.disabled = false;
    pauseBtn.disabled = true;
};

function reasonToString(reasonCode) {
    switch (reasonCode) {
        case 0: return 'limit reached (more solutions may exist)';
        case 1: return 'search exhausted (no more solutions)';
        case 2: return 'max steps reached (result inconclusive)';
        case -1: return 'no query executed';
        default: return 'unknown';
    }
}

function updateStateVars() {
    const count = frontend_state_var_count(frontend);
    let html = '';
    if (lastRunStage >= 0) {
        const stageName = frontend_stage_name(frontend, lastRunStage);
        html += `<strong>Active Stage:</strong> ${stageName} <button id="clear-stage-btn">Clear</button><br/><br/>`;
    }
    if (count === 0) {
        html += '<em>No state variables</em>';
    } else {
        html += '<strong>State Variables:</strong><br/>';
        for (let i = 0; i < count; i++) {
            const name = frontend_state_var_name(frontend, i);
            const value = frontend_state_var_value(frontend, i);
            html += `  ${name} = ${value}<br/>`;
        }
    }
    stateVarsDiv.innerHTML = html;
    const clearBtn = document.querySelector('#clear-stage-btn');
    if (clearBtn) {
        clearBtn.onclick = () => {
            lastRunStage = -1;
            updateStateVars();
        };
    }
}

strategySelect.onchange = () => {
    frontend_set_strategy(frontend, parseInt(strategySelect.value));
};

maxStepsInput.onchange = () => {
    frontend_set_max_steps(frontend, parseInt(maxStepsInput.value) || 10000);
};

function buildStageButtons() {
    stagesSpan.innerHTML = '';
    const count = frontend_stage_count(frontend);
    for (let i = 0; i < count; i++) {
        const stageIndex = i;
        const btn = document.createElement('button');
        btn.textContent = frontend_stage_name(frontend, i);
        btn.onclick = () => {
            const result = frontend_run_stage(frontend, stageIndex);
            if (result === 1) {
                lastRunStage = stageIndex;
                out.innerText += '\nRan stage: ' + btn.textContent;
                updateStateVars();
                drawRects();
            } else {
                out.innerText += '\nStage failed: ' + btn.textContent;
            }
        };
        stagesSpan.appendChild(btn);
    }
}

reloadBtn.onclick = () => {
    if (currentModule) {
        free_module(currentModule);
    }

    const code = inp.value;
    currentModule = parse_module(code);

    if (!currentModule) {
        out.innerText = 'Parse error';
        return;
    }

    const loadResult = frontend_load(frontend, code);
    if (loadResult !== 0) {
        out.innerText = 'Load error';
        return;
    }

    lastRunStage = -1;
    const factCount = frontend_fact_count(frontend);
    const ruleCount = frontend_rule_count(frontend);
    const stageCount = frontend_stage_count(frontend);

    out.innerText = 'Loaded: ' + factCount + ' facts, ' + ruleCount + ' global rules, ' + stageCount + ' stages';

    buildStageButtons();
    findStageIndices();
    updateStateVars();
    drawRects();
};

function updateButtons() {
    const hasMore = frontend_has_more(frontend) !== 0;
    nextBtn.disabled = !hasMore;
    stopBtn.disabled = !hasMore;
}

queryBtn.onclick = () => {
    const queryStr = queryInput.value;
    if (!queryStr) return;
    frontend_query_stop(frontend);
    queryStartTime = performance.now();
    const result = frontend_query_start(frontend, queryStr, lastRunStage);
    const elapsed = (performance.now() - queryStartTime).toFixed(2);
    const reasonCode = frontend_query_reason(frontend);
    const reason = reasonToString(reasonCode);
    const stageInfo = lastRunStage >= 0 ? ` (in stage: ${frontend_stage_name(frontend, lastRunStage)})` : '';
    out.innerText += '\n\nQuery: ' + queryStr + stageInfo + '\n' + result + '\n(Time: ' + elapsed + 'ms, Reason: ' + reason + ')';
    updateButtons();
};

nextBtn.onclick = () => {
    const result = frontend_query_next(frontend);
    const elapsed = (performance.now() - queryStartTime).toFixed(2);
    out.innerText += '\n' + result + '\n(Time: ' + elapsed + 'ms)';
    updateButtons();
};

stopBtn.onclick = () => {
    frontend_query_stop(frontend);
    out.innerText += '\n(stopped)';
    updateButtons();
};

reloadBtn.click();
