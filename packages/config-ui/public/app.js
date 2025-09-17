/* eslint-env browser */

class ConfigWizard {
  constructor() {
    this.currentStep = 0;
    this.steps = ['providers', 'models', 'agents', 'review'];
    this.config = null;
    this.availableModels = [];

    this.init();
  }

  async init() {
    await this.loadConfig();
    this.initEventListeners();
    this.showWizard();
    this.populateExistingValues();
  }

  async loadConfig() {
    try {
      const response = await fetch('/api/config');
      if (response.ok) {
        this.config = await response.json();
      } else {
        this.config = this.getDefaultConfig();
      }
    } catch (error) {
      console.error('Failed to load config:', error);
      this.config = this.getDefaultConfig();
    }
  }

  getDefaultConfig() {
    return {
      version: '1.0.0',
      default_model: 'anthropic.haiku',
      providers: {
        anthropic: { models: {} },
        openai: { models: {} },
        lmstudio: { models: {} },
      },
      agents: {
        critic: {
          enabled: true,
          model: 'anthropic.haiku',
          temperature: 0.3,
          max_tokens: 2000,
        },
        actor: {
          enabled: true,
          model: 'default',
          temperature: 0.7,
          max_tokens: 4000,
        },
      },
    };
  }

  showWizard() {
    document.getElementById('loading').style.display = 'none';
    document.getElementById('config-wizard').style.display = 'block';
  }

  initEventListeners() {
    // Test connection buttons
    document.querySelectorAll('.test-connection').forEach((btn) => {
      btn.addEventListener('click', (e) => this.testConnection(e.target.dataset.provider));
    });

    // Input change listeners for real-time testing
    document.getElementById('lmstudio-url').addEventListener(
      'input',
      this.debounce((e) => this.handleLMStudioUrlChange(e.target.value), 1000),
    );

    // Navigation buttons
    document.getElementById('next-btn').addEventListener('click', () => this.nextStep());
    document.getElementById('prev-btn').addEventListener('click', () => this.prevStep());
    document.getElementById('save-btn').addEventListener('click', () => this.saveConfig());

    // Range inputs
    document.querySelectorAll('input[type="range"]').forEach((range) => {
      range.addEventListener('input', (e) => {
        const valueSpan = e.target.parentElement.querySelector('.range-value');
        valueSpan.textContent = e.target.value;
      });
    });
  }

  populateExistingValues() {
    // Populate provider configurations
    if (this.config.providers.lmstudio?.base_url) {
      document.getElementById('lmstudio-url').value = this.config.providers.lmstudio.base_url;
      this.testConnection('lmstudio');
    }

    if (this.config.providers.anthropic?.api_key) {
      document.getElementById('anthropic-key').value = this.config.providers.anthropic.api_key;
      this.updateProviderStatus('anthropic', 'success', 'Connected');
    }

    if (this.config.providers.openai?.api_key) {
      document.getElementById('openai-key').value = this.config.providers.openai.api_key;
      this.updateProviderStatus('openai', 'success', 'Connected');
    }

    // Populate agent settings
    if (this.config.agents) {
      const critic = this.config.agents.critic;
      if (critic) {
        document.getElementById('critic-enabled').checked = critic.enabled;
        document.getElementById('critic-temperature').value = critic.temperature;
        document.querySelector('#critic-temperature + .range-value').textContent =
          critic.temperature;
        document.getElementById('critic-max-tokens').value = critic.max_tokens || 2000;
      }

      const actor = this.config.agents.actor;
      if (actor) {
        document.getElementById('actor-enabled').checked = actor.enabled;
        document.getElementById('actor-temperature').value = actor.temperature;
        document.querySelector('#actor-temperature + .range-value').textContent = actor.temperature;
        document.getElementById('actor-max-tokens').value = actor.max_tokens || 4000;
      }
    }
  }

  async testConnection(provider) {
    const btn = document.querySelector(`[data-provider="${provider}"].test-connection`);
    const btnText = btn.querySelector('.btn-text');
    const btnSpinner = btn.querySelector('.btn-spinner');

    // Show loading state
    btnText.style.display = 'none';
    btnSpinner.style.display = 'inline-block';
    btn.disabled = true;

    this.updateProviderStatus(provider, 'testing', 'Testing connection...');

    try {
      const providerConfig = {};

      if (provider === 'lmstudio') {
        const url = document.getElementById('lmstudio-url').value;
        if (!url) {
          throw new Error('Please enter a base URL');
        }
        providerConfig.base_url = url;
      } else if (provider === 'anthropic') {
        const apiKey = document.getElementById('anthropic-key').value;
        if (!apiKey) {
          throw new Error('Please enter an API key');
        }
        providerConfig.api_key = apiKey;
      } else if (provider === 'openai') {
        const apiKey = document.getElementById('openai-key').value;
        if (!apiKey) {
          throw new Error('Please enter an API key');
        }
        providerConfig.api_key = apiKey;
      }

      const response = await fetch(`/api/providers/${provider}/test`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(providerConfig),
      });

      const result = await response.json();

      if (result.success) {
        this.updateProviderStatus(provider, 'success', 'Connected');

        if (result.models && result.models.length > 0) {
          this.displayProviderModels(provider, result.models);
          this.updateAvailableModels();
        }

        // Update config
        if (!this.config.providers[provider]) {
          this.config.providers[provider] = { models: {} };
        }
        Object.assign(this.config.providers[provider], providerConfig);

        // Add discovered models to config
        if (result.models) {
          result.models.forEach((model) => {
            const modelKey = model.id.replace(/[^a-zA-Z0-9_-]/g, '_');
            this.config.providers[provider].models[modelKey] = {
              id: model.id,
              description: `${provider} model: ${model.id}`,
            };
          });
        }
      } else {
        throw new Error(result.error || 'Connection failed');
      }
    } catch (error) {
      this.updateProviderStatus(provider, 'error', error.message);
    } finally {
      // Hide loading state
      btnText.style.display = 'inline';
      btnSpinner.style.display = 'none';
      btn.disabled = false;
    }
  }

  updateProviderStatus(provider, status, text) {
    const statusElement = document.getElementById(`${provider}-status`);
    const dot = statusElement.querySelector('.status-dot');
    const textElement = statusElement.querySelector('.status-text');

    dot.className = `status-dot status-${status}`;
    textElement.textContent = text;

    const card = document.querySelector(`[data-provider="${provider}"]`);
    if (status === 'success') {
      card.classList.add('connected');
    } else {
      card.classList.remove('connected');
    }
  }

  displayProviderModels(provider, models) {
    const modelsContainer = document.getElementById(`${provider}-models`);
    const modelsList = modelsContainer.querySelector('.models-list');

    modelsList.innerHTML = '';
    models.forEach((model) => {
      const tag = document.createElement('div');
      tag.className = 'model-tag';
      tag.textContent = model.id;
      modelsList.appendChild(tag);
    });

    modelsContainer.style.display = 'block';
  }

  async handleLMStudioUrlChange(url) {
    if (url && this.isValidUrl(url)) {
      await this.testConnection('lmstudio');
    }
  }

  isValidUrl(string) {
    try {
      new URL(string);
      return true;
    } catch (_) {
      return false;
    }
  }

  updateAvailableModels() {
    this.availableModels = [];

    // Get all models from all providers
    Object.entries(this.config.providers).forEach(([provider, config]) => {
      if (config.models) {
        Object.entries(config.models).forEach(([key, model]) => {
          this.availableModels.push({
            id: `${provider}.${key}`,
            name: model.description || `${provider} - ${key}`,
            provider,
            modelId: model.id,
          });
        });
      }
    });

    // Update model selection dropdowns
    this.updateModelSelects();
    this.updateModelsGrid();
  }

  updateModelSelects() {
    const selects = ['critic-model', 'actor-model'];

    selects.forEach((selectId) => {
      const select = document.getElementById(selectId);
      const currentValue = select.value;

      select.innerHTML = '<option value="">Select a model...</option>';

      this.availableModels.forEach((model) => {
        const option = document.createElement('option');
        option.value = model.id;
        option.textContent = model.name;
        select.appendChild(option);
      });

      // Restore previous selection if still valid
      if (currentValue && this.availableModels.some((m) => m.id === currentValue)) {
        select.value = currentValue;
      }
    });
  }

  updateModelsGrid() {
    const grid = document.getElementById('models-grid');
    grid.innerHTML = '';

    this.availableModels.forEach((model) => {
      const card = document.createElement('div');
      card.className = 'model-card';
      card.dataset.modelId = model.id;

      if (model.id === this.config.default_model) {
        card.classList.add('selected');
      }

      card.innerHTML = `
                <h3>${model.modelId}</h3>
                <p>${model.name}</p>
                <span class="model-provider">${model.provider}</span>
            `;

      card.addEventListener('click', () => this.selectDefaultModel(model.id));
      grid.appendChild(card);
    });
  }

  selectDefaultModel(modelId) {
    // Update selection visually
    document.querySelectorAll('.model-card').forEach((card) => {
      card.classList.remove('selected');
    });
    document.querySelector(`[data-model-id="${modelId}"]`).classList.add('selected');

    // Update config
    this.config.default_model = modelId;
  }

  nextStep() {
    if (this.currentStep < this.steps.length - 1) {
      this.currentStep++;
      this.updateStep();

      // Special handling for models step
      if (this.steps[this.currentStep] === 'models') {
        this.updateAvailableModels();
      }

      // Special handling for review step
      if (this.steps[this.currentStep] === 'review') {
        this.updateConfigPreview();
      }
    }
  }

  prevStep() {
    if (this.currentStep > 0) {
      this.currentStep--;
      this.updateStep();
    }
  }

  updateStep() {
    // Update step indicators
    document.querySelectorAll('.step').forEach((step, index) => {
      step.classList.remove('active', 'completed');
      if (index === this.currentStep) {
        step.classList.add('active');
      } else if (index < this.currentStep) {
        step.classList.add('completed');
      }
    });

    // Update step panels
    document.querySelectorAll('.step-panel').forEach((panel, index) => {
      panel.classList.remove('active');
      if (index === this.currentStep) {
        panel.classList.add('active');
      }
    });

    // Update navigation buttons
    const prevBtn = document.getElementById('prev-btn');
    const nextBtn = document.getElementById('next-btn');
    const saveBtn = document.getElementById('save-btn');

    prevBtn.style.display = this.currentStep === 0 ? 'none' : 'inline-flex';

    if (this.currentStep === this.steps.length - 1) {
      nextBtn.style.display = 'none';
      saveBtn.style.display = 'inline-flex';
    } else {
      nextBtn.style.display = 'inline-flex';
      saveBtn.style.display = 'none';
    }
  }

  updateConfigPreview() {
    // Collect all form data
    const formConfig = {
      version: this.config.version,
      default_model: this.config.default_model,
      providers: {
        ...this.config.providers,
      },
      agents: {
        critic: {
          enabled: document.getElementById('critic-enabled').checked,
          model: document.getElementById('critic-model').value || this.config.default_model,
          temperature: parseFloat(document.getElementById('critic-temperature').value),
          max_tokens: parseInt(document.getElementById('critic-max-tokens').value),
        },
        actor: {
          enabled: document.getElementById('actor-enabled').checked,
          model: document.getElementById('actor-model').value || this.config.default_model,
          temperature: parseFloat(document.getElementById('actor-temperature').value),
          max_tokens: parseInt(document.getElementById('actor-max-tokens').value),
        },
      },
    };

    // Update providers with form values
    const lmstudioUrl = document.getElementById('lmstudio-url').value;
    if (lmstudioUrl) {
      formConfig.providers.lmstudio.base_url = lmstudioUrl;
    }

    const anthropicKey = document.getElementById('anthropic-key').value;
    if (anthropicKey) {
      formConfig.providers.anthropic.api_key = anthropicKey;
    }

    const openaiKey = document.getElementById('openai-key').value;
    if (openaiKey) {
      formConfig.providers.openai.api_key = openaiKey;
    }

    // Update preview
    document.getElementById('config-json').textContent = JSON.stringify(formConfig, null, 2);

    // Store for saving
    this.finalConfig = formConfig;
  }

  async saveConfig() {
    const saveBtn = document.getElementById('save-btn');
    const originalText = saveBtn.textContent;

    saveBtn.textContent = 'Saving...';
    saveBtn.disabled = true;

    try {
      const response = await fetch('/api/config', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(this.finalConfig),
      });

      if (response.ok) {
        this.showSuccessState();
      } else {
        throw new Error('Failed to save configuration');
      }
    } catch (error) {
      alert(`Failed to save configuration: ${error.message}`);
      saveBtn.textContent = originalText;
      saveBtn.disabled = false;
    }
  }

  showSuccessState() {
    document.getElementById('config-wizard').style.display = 'none';
    document.getElementById('success-state').style.display = 'flex';
  }

  debounce(func, wait) {
    let timeout;
    return function executedFunction(...args) {
      const later = () => {
        clearTimeout(timeout);
        func(...args);
      };
      clearTimeout(timeout);
      timeout = setTimeout(later, wait);
    };
  }
}

// Initialize the wizard when the page loads
document.addEventListener('DOMContentLoaded', () => {
  new ConfigWizard();
});
