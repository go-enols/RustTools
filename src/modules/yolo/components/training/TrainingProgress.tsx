import { TrainingMetrics, BatchProgress } from '../../../../core/stores/trainingStore';
import styles from '../../pages/TrainingPage.module.css';

interface TrainingProgressProps {
  currentEpoch: number;
  totalEpochs: number;
  elapsedTime: string;
  remainingTime: string;
  metrics: TrainingMetrics[];
  batchProgress: BatchProgress | null;
}

export default function TrainingProgress({
  currentEpoch,
  totalEpochs,
  elapsedTime,
  remainingTime,
  metrics,
  batchProgress,
}: TrainingProgressProps) {
  const progressPercent = totalEpochs > 0 ? ((currentEpoch / totalEpochs) * 100).toFixed(1) : 0;

  const latestMetric = metrics.length > 0 ? metrics[metrics.length - 1] : null;

  return (
    <div className={styles.cardHighlighted}>
      {/* Epoch Progress */}
      <div className={styles.progressHeader}>
        <span className={styles.progressLabel}>
          总进度 Epoch {currentEpoch} / {totalEpochs}
        </span>
        <span className={styles.progressPercent}>{progressPercent}%</span>
      </div>
      <div className={styles.progressBar}>
        <div className={styles.progressFill} style={{ width: `${progressPercent}%` }} />
      </div>

      {/* Batch Progress */}
      {batchProgress && batchProgress.batch > 0 && (
        <div className={styles.batchProgress}>
          <div className={styles.batchHeader}>
            <span className={styles.batchLabel}>
              当前 Epoch 进度 Batch {batchProgress.batch}
              {batchProgress.totalBatches > 0 ? ` / ${batchProgress.totalBatches}` : ''}
            </span>
            <span className={styles.batchPercent}>
              {batchProgress.totalBatches > 0
                ? `${((batchProgress.batch / batchProgress.totalBatches) * 100).toFixed(1)}%`
                : `${batchProgress.batch} batches`}
            </span>
          </div>
          <div className={styles.batchBar}>
            <div
              className={styles.batchFill}
              style={{
                width: `${
                  batchProgress.totalBatches > 0
                    ? Math.min((batchProgress.batch / batchProgress.totalBatches) * 100, 100)
                    : 0
                }%`,
              }}
            />
          </div>
        </div>
      )}

      {/* Stats Grid */}
      <div className={styles.statsGrid}>
        <div className={styles.statCard}>
          <div className={styles.statLabel}>已用时间</div>
          <div className={styles.statValue}>{elapsedTime}</div>
        </div>
        <div className={styles.statCard}>
          <div className={styles.statLabel}>预计剩余</div>
          <div className={styles.statValue}>{remainingTime}</div>
        </div>

        {latestMetric && (
          <>
            <div className={styles.statCard}>
              <div className={styles.statLabel}>Box Loss</div>
              <div className={styles.statValue}>{latestMetric.trainBoxLoss.toFixed(4)}</div>
            </div>
            <div className={styles.statCard}>
              <div className={styles.statLabel}>Cls Loss</div>
              <div className={styles.statValue}>{latestMetric.trainClsLoss.toFixed(4)}</div>
            </div>
          </>
        )}

        {batchProgress && (
          <>
            <div className={styles.statCard}>
              <div className={styles.statLabel}>Box Loss</div>
              <div className={styles.statValue}>{batchProgress.boxLoss.toFixed(4)}</div>
            </div>
            <div className={styles.statCard}>
              <div className={styles.statLabel}>Cls Loss</div>
              <div className={styles.statValue}>{batchProgress.clsLoss.toFixed(4)}</div>
            </div>
          </>
        )}

        {latestMetric && latestMetric.map50 > 0 && (
          <>
            <div className={styles.statCard}>
              <div className={styles.statLabel}>mAP50</div>
              <div className={styles.statValueAccent}>{(latestMetric.map50 * 100).toFixed(1)}%</div>
            </div>
            <div className={styles.statCard}>
              <div className={styles.statLabel}>Precision</div>
              <div className={styles.statValue}>{(latestMetric.precision * 100).toFixed(1)}%</div>
            </div>
          </>
        )}
      </div>
    </div>
  );
}
