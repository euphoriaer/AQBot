import { Form, Input, InputNumber, Select, Slider, Switch, Typography, theme } from 'antd';
import { ChevronDown, ChevronRight } from 'lucide-react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import {
  getDrawingParamConfig,
  getDrawingModelOptions,
  getDrawingProvidersForModel,
  normalizeDrawingSettingsByConfig,
} from '@/lib/drawingModels';
import { SmartProviderIcon } from '@/lib/providerIcons';
import type { DrawingSettings, ProviderConfig } from '@/types';
import { DrawingReferenceUploader } from './DrawingReferenceUploader';
import type {
  DrawingParamField,
  DrawingParamOption,
  DrawingParamRenderContext,
} from './params/types';

export type { DrawingSettings };

interface Props {
  settings: DrawingSettings;
  providers: ProviderConfig[];
  onChange: (settings: DrawingSettings) => void;
}

export function DrawingSettingsPanel({ settings, providers, onChange }: Props) {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const [advancedOpen, setAdvancedOpen] = useState(false);
  const translateOption = (key: string, fallback: string) => t(key, fallback);
  const paramConfig = getDrawingParamConfig(settings.modelId);
  const basicFields = paramConfig.groups.find((group) => group.id === 'basic')?.fields ?? [];
  const advancedFields = paramConfig.groups.find((group) => group.id === 'advanced')?.fields ?? [];
  const modelOptions = getDrawingModelOptions();
  const compatibleProviders = getDrawingProvidersForModel(providers, settings.modelId);
  const paramModelOptions: DrawingParamOption[] = modelOptions.map((option) => ({
    fallbackLabel: option.label,
    value: option.value,
  }));
  const paramProviderOptions: DrawingParamOption[] = compatibleProviders.map((provider) => ({
    fallbackLabel: provider.name,
    value: provider.id,
  }));
  const providerSelectOptions = compatibleProviders.map((provider) => ({
    label: (
      <span className="inline-flex items-center gap-2">
        <SmartProviderIcon provider={provider} size={18} type="avatar" />
        <span>{provider.name}</span>
      </span>
    ),
    value: provider.id,
  }));

  const renderContext: DrawingParamRenderContext = {
    settings,
    providers,
    modelOptions: paramModelOptions,
    providerOptions: paramProviderOptions,
    t: translateOption,
    getProvidersForModel: (modelId) => getDrawingProvidersForModel(providers, modelId),
  };

  const visibleBasicFields = basicFields.filter((field) => isFieldVisible(field, renderContext));
  const visibleAdvancedFields = advancedFields.filter((field) => isFieldVisible(field, renderContext));

  const patch = (next: Partial<DrawingSettings>) => {
    onChange(normalizeDrawingSettingsByConfig({ ...settings, ...next }));
  };

  const patchField = (field: DrawingParamField, value: unknown) => {
    const next = field.normalizeOnChange
      ? field.normalizeOnChange(value, renderContext)
      : field.key
        ? ({ [field.key]: value } as Partial<DrawingSettings>)
        : {};
    patch(next);
  };

  const renderField = (field: DrawingParamField) => {
    const label = t(field.labelKey, field.fallbackLabel);
    switch (field.type) {
      case 'modelSelect':
        return (
          <Form.Item key={field.id} label={label}>
            <Select
              value={settings.modelId}
              options={toSelectOptions(paramModelOptions, translateOption)}
              placeholder={t('drawing.selectModel', '选择绘图模型')}
              onChange={(modelId) => patchField(field, modelId)}
            />
          </Form.Item>
        );
      case 'providerSelect':
        return (
          <Form.Item key={field.id} label={label}>
            <Select
              value={settings.providerId || undefined}
              placeholder={t('drawing.selectProvider', '选择服务商')}
              options={providerSelectOptions}
              optionLabelProp="label"
              onChange={(providerId) => patchField(field, providerId)}
            />
          </Form.Item>
        );
      case 'select':
        return (
          <Form.Item key={field.id} label={label}>
            <Select
              value={field.key ? settings[field.key] : undefined}
              options={toSelectOptions(resolveOptions(field, renderContext), translateOption)}
              onChange={(value) => patchField(field, value)}
            />
          </Form.Item>
        );
      case 'number':
        return (
          <Form.Item key={field.id} label={label}>
            <InputNumber
              min={field.min}
              max={field.max}
              value={field.key ? Number(settings[field.key]) : field.defaultValue}
              style={{ width: '100%' }}
              onChange={(value) => patchField(field, value ?? field.defaultValue ?? field.min ?? 0)}
            />
          </Form.Item>
        );
      case 'text':
        return (
          <Form.Item key={field.id} label={label}>
            <Input
              value={field.key ? String(settings[field.key] ?? '') : ''}
              placeholder={field.placeholder}
              onChange={(event) => patchField(field, event.target.value)}
            />
            {field.fallbackHint && (
              <Typography.Text type="secondary" style={{ fontSize: 12 }}>
                {t(field.hintKey ?? field.id, field.fallbackHint)}
              </Typography.Text>
            )}
          </Form.Item>
        );
      case 'compression':
        return (
          <Form.Item key={field.id} label={label}>
            <div className="flex items-center gap-3">
              <Switch
                checked={settings.outputCompression !== undefined}
                onChange={(checked) => patchField(field, checked ? field.defaultValue ?? 90 : undefined)}
              />
              <Slider
                min={field.min ?? 0}
                max={field.max ?? 100}
                disabled={settings.outputCompression === undefined}
                value={settings.outputCompression ?? field.defaultValue ?? 90}
                onChange={(outputCompression) => patchField(field, outputCompression)}
                style={{ flex: 1 }}
              />
            </div>
          </Form.Item>
        );
      case 'referenceUploader':
        return (
          <div key={field.id}>
            <Typography.Text style={{ fontSize: 12, color: token.colorTextSecondary }}>
              {label}
            </Typography.Text>
            <div className="mb-4 mt-2">
              <DrawingReferenceUploader />
            </div>
          </div>
        );
      default:
        return null;
    }
  };

  return (
    <aside
      className="h-full overflow-y-auto"
      style={{
        width: 304,
        borderRight: `1px solid ${token.colorBorderSecondary}`,
        background: token.colorBgContainer,
        padding: 16,
      }}
    >
      <Form layout="vertical">
        {visibleBasicFields.map(renderField)}
      </Form>
      {visibleAdvancedFields.length > 0 && (
        <>
          <button
            type="button"
            onClick={() => setAdvancedOpen((open) => !open)}
            aria-expanded={advancedOpen}
            className="mb-3 flex w-full items-center justify-between transition-colors"
            style={{
              height: 44,
              border: 'none',
              borderTop: `1px solid ${token.colorBorderSecondary}`,
              background: 'transparent',
              color: token.colorText,
              padding: 0,
              fontSize: 14,
              fontWeight: 600,
              textAlign: 'left',
            }}
          >
            <span>{t('drawing.advancedSettings', '高级设置')}</span>
            <span
              className="flex items-center justify-center"
              style={{
                width: 24,
                height: 24,
                borderRadius: 6,
                background: token.colorFillAlter,
                color: token.colorTextSecondary,
              }}
            >
              {advancedOpen ? <ChevronDown size={15} /> : <ChevronRight size={15} />}
            </span>
          </button>
          {advancedOpen && (
            <Form layout="vertical">
              {visibleAdvancedFields.map(renderField)}
            </Form>
          )}
        </>
      )}
    </aside>
  );
}

function isFieldVisible(field: DrawingParamField, context: DrawingParamRenderContext): boolean {
  return field.visibleWhen ? field.visibleWhen(context) : true;
}

function resolveOptions(
  field: DrawingParamField,
  context: DrawingParamRenderContext,
): readonly DrawingParamOption[] {
  if (!field.options) return [];
  return typeof field.options === 'function' ? field.options(context) : field.options;
}

function toSelectOptions(options: readonly DrawingParamOption[], t: (key: string, fallback: string) => string) {
  return options.map((option) => ({
    label: option.labelKey ? t(option.labelKey, option.fallbackLabel) : option.fallbackLabel,
    value: option.value,
  }));
}
