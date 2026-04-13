import { TrainingMetrics } from '../../../../core/stores/trainingStore';
import styles from '../../pages/TrainingPage.module.css';

interface TrainingLogsProps {
  metrics: TrainingMetrics[];
}

export default function TrainingLogs({ metrics }: TrainingLogsProps) {
  return (
    <div className={styles.card}>
      <div className={styles.cardHeader}>
        <span className={styles.cardTitle}>训练日志</span>
      </div>
      <div className={styles.logPanel}>
        {metrics.length === 0 ? (
          <div className={styles.logEntry}>等待开始训练...</div>
        ) : (
          metrics.slice(-20).map((m, i) => (
            <div key={i} className={styles.logEntry}>
              Epoch {m.epoch}: box_loss={m.trainBoxLoss.toFixed(4)}, cls_loss={m.trainClsLoss.toFixed(4)},
              mAP50={m.map50.toFixed(4)}, precision={m.precision.toFixed(4)}, recall={m.recall.toFixed(4)}
            </div>
          ))
        )}
      </div>
    </div>
  );
}
