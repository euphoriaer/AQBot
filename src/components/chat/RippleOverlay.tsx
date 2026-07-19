import { useTranslation } from 'react-i18next';
import { Radio } from 'lucide-react';
import styles from './RippleOverlay.module.css';

interface RippleOverlayProps {
  active: boolean;
  sourceAddress?: string | null;
}

export function RippleOverlay({ active, sourceAddress }: RippleOverlayProps) {
  const { t } = useTranslation();

  if (!active) return null;

  return (
    <div className={styles.rippleOverlay}>
      <div className={styles.rippleRing} />
      <div className={styles.rippleRing} />
      <div className={styles.rippleRing} />
      <div className={styles.rippleRing} />
      {sourceAddress && (
        <div className={styles.rippleHint}>
          <Radio size={11} />
          <span>
            {sourceAddress}
            {' '}
            {t('chat.sessionInterop.remoteInputHint')}
          </span>
        </div>
      )}
    </div>
  );
}
