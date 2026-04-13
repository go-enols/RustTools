import { Play, Square } from 'lucide-react';
import styles from '../../pages/TrainingPage.module.css';

interface TrainingHeaderProps {
  isTraining: boolean;
  onStartTraining: () => void;
  onStopTraining: () => void;
}

export default function TrainingHeader({ isTraining, onStartTraining, onStopTraining }: TrainingHeaderProps) {
  return (
    <div className={styles.header}>
      <div className={styles.headerLeft}>
        <h1 className={styles.title}>模型训练</h1>
        <p className={styles.subtitle}>训练YOLO目标检测模型</p>
      </div>
      <div className={styles.headerRight}>
        {!isTraining ? (
          <button className={`${styles.btn} ${styles.btnPrimary}`} onClick={onStartTraining}>
            <Play size={16} />
            开始训练
          </button>
        ) : (
          <button className={`${styles.btn} ${styles.btnDanger}`} onClick={onStopTraining}>
            <Square size={16} />
            停止训练
          </button>
        )}
      </div>
    </div>
  );
}
