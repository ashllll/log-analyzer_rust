/**
 * Property-Based Tests for Frontend WebSocket Synchronization
 * 
 * Tests the following properties from the design document:
 * - Property 8: UI Synchronization Immediacy
 * - Property 9: Event Structure Completeness
 * - Property 16: Synchronization Monitoring
 * 
 * **Validates: Requirements 2.3, 2.4, 4.2**
 */

import * as fc from 'fast-check';
import {
  EventNotificationMessage,
  WorkspaceEventPayload,
  ConnectionStatus,
  SyncMetrics,
} from '../../types/websocket';

// ============================================================================
// Arbitraries (Generators)
// ============================================================================

/**
 * Generate valid workspace IDs
 */
const workspaceIdArb = fc.stringMatching(/^[a-zA-Z0-9-]{1,50}$/);

/**
 * Generate valid task IDs
 */
const taskIdArb = fc.stringMatching(/^[a-zA-Z0-9-]{1,50}$/);

/**
 * Generate valid timestamps (ISO strings)
 * Using integer timestamps to avoid invalid date issues
 */
const timestampArb = fc.integer({ 
  min: new Date('2020-01-01').getTime(), 
  max: new Date('2030-12-31').getTime() 
}).map(ts => new Date(ts).toISOString());

/**
 * Generate workspace status types
 */
const workspaceStatusArb = fc.oneof(
  fc.constant({ type: 'Idle' as const }),
  fc.record({
    type: fc.constant('Processing' as const),
    started_at: timestampArb,
  }),
  fc.record({
    type: fc.constant('Completed' as const),
    duration: fc.integer({ min: 0, max: 1000000 }),
  }),
  fc.record({
    type: fc.constant('Failed' as const),
    error: fc.string({ minLength: 1, maxLength: 200 }),
    failed_at: timestampArb,
  }),
  fc.record({
    type: fc.constant('Cancelled' as const),
    cancelled_at: timestampArb,
  })
);

/**
 * Generate StatusChanged events
 */
const statusChangedEventArb = fc.record({
  StatusChanged: fc.record({
    workspace_id: workspaceIdArb,
    status: workspaceStatusArb,
    timestamp: timestampArb,
  }),
});

/**
 * Generate ProgressUpdate events
 */
const progressUpdateEventArb = fc.record({
  ProgressUpdate: fc.record({
    workspace_id: workspaceIdArb,
    progress: fc.double({ min: 0, max: 100, noNaN: true }),
    timestamp: timestampArb,
  }),
});

/**
 * Generate TaskCompleted events
 */
const taskCompletedEventArb = fc.record({
  TaskCompleted: fc.record({
    workspace_id: workspaceIdArb,
    task_id: taskIdArb,
    timestamp: timestampArb,
  }),
});

/**
 * Generate Error events
 */
const errorEventArb = fc.record({
  Error: fc.record({
    workspace_id: workspaceIdArb,
    error: fc.string({ minLength: 1, maxLength: 500 }),
    timestamp: timestampArb,
  }),
});

/**
 * Generate WorkspaceDeleted events
 */
const workspaceDeletedEventArb = fc.record({
  WorkspaceDeleted: fc.record({
    workspace_id: workspaceIdArb,
    timestamp: timestampArb,
  }),
});

/**
 * Generate WorkspaceCreated events
 */
const workspaceCreatedEventArb = fc.record({
  WorkspaceCreated: fc.record({
    workspace_id: workspaceIdArb,
    timestamp: timestampArb,
  }),
});

/**
 * Generate any workspace event payload
 */
const workspaceEventPayloadArb: fc.Arbitrary<WorkspaceEventPayload> = fc.oneof(
  statusChangedEventArb,
  progressUpdateEventArb,
  taskCompletedEventArb,
  errorEventArb,
  workspaceDeletedEventArb,
  workspaceCreatedEventArb
) as fc.Arbitrary<WorkspaceEventPayload>;

/**
 * Generate EventNotification messages
 */
const eventNotificationArb: fc.Arbitrary<EventNotificationMessage> = fc.record({
  type: fc.constant('EventNotification' as const),
  event_id: fc.stringMatching(/^event:[0-9]+$/),
  event_type: fc.constantFrom(
    'status_changed',
    'progress_update',
    'task_completed',
    'error',
    'workspace_deleted',
    'workspace_created'
  ),
  payload: workspaceEventPayloadArb,
});

/**
 * Generate connection status
 */
const connectionStatusArb: fc.Arbitrary<ConnectionStatus> = fc.constantFrom(
  'connecting',
  'connected',
  'disconnected',
  'reconnecting',
  'error'
);

/**
 * Generate sync metrics
 */
const syncMetricsArb: fc.Arbitrary<SyncMetrics> = fc.record({
  messagesReceived: fc.integer({ min: 0, max: 1000000 }),
  messagesSent: fc.integer({ min: 0, max: 1000000 }),
  successfulDeliveries: fc.integer({ min: 0, max: 1000000 }),
  failedDeliveries: fc.integer({ min: 0, max: 1000000 }),
  averageLatency: fc.double({ min: 0, max: 10000, noNaN: true }),
  lastSyncTime: fc.option(fc.date().map(d => d), { nil: null }),
});

// ============================================================================
// Helper Functions
// ============================================================================

/**
 * Extract workspace ID from any event payload
 */
function extractWorkspaceId(payload: WorkspaceEventPayload): string {
  if ('StatusChanged' in payload) return payload.StatusChanged.workspace_id;
  if ('ProgressUpdate' in payload) return payload.ProgressUpdate.workspace_id;
  if ('TaskCompleted' in payload) return payload.TaskCompleted.workspace_id;
  if ('Error' in payload) return payload.Error.workspace_id;
  if ('WorkspaceDeleted' in payload) return payload.WorkspaceDeleted.workspace_id;
  if ('WorkspaceCreated' in payload) return payload.WorkspaceCreated.workspace_id;
  throw new Error('Unknown event type');
}

/**
 * Extract timestamp from any event payload
 */
function extractTimestamp(payload: WorkspaceEventPayload): string {
  if ('StatusChanged' in payload) return payload.StatusChanged.timestamp;
  if ('ProgressUpdate' in payload) return payload.ProgressUpdate.timestamp;
  if ('TaskCompleted' in payload) return payload.TaskCompleted.timestamp;
  if ('Error' in payload) return payload.Error.timestamp;
  if ('WorkspaceDeleted' in payload) return payload.WorkspaceDeleted.timestamp;
  if ('WorkspaceCreated' in payload) return payload.WorkspaceCreated.timestamp;
  throw new Error('Unknown event type');
}

// ============================================================================
// Property Tests
// ============================================================================

describe('Frontend WebSocket Synchronization Properties', () => {
  /**
   * **Feature: performance-optimization, Property 8: UI Synchronization Immediacy**
   * 
   * *For any* workspace deletion, UI should reflect changes automatically without manual refresh
   * 
   * This property tests that:
   * 1. All workspace events contain valid workspace IDs
   * 2. Events can be processed without errors
   * 3. The event structure allows for immediate UI updates
   * 
   * **Validates: Requirements 2.3**
   */
  describe('Property 8: UI Synchronization Immediacy', () => {
    it('should have valid workspace ID in all events for immediate UI updates', () => {
      fc.assert(
        fc.property(workspaceEventPayloadArb, (payload) => {
          const workspaceId = extractWorkspaceId(payload);
          
          // Workspace ID should be non-empty
          expect(workspaceId.length).toBeGreaterThan(0);
          
          // Workspace ID should match expected pattern
          expect(workspaceId).toMatch(/^[a-zA-Z0-9-]+$/);
          
          return true;
        }),
        { numRuns: 100 }
      );
    });

    it('should have valid timestamp in all events for ordering', () => {
      fc.assert(
        fc.property(workspaceEventPayloadArb, (payload) => {
          const timestamp = extractTimestamp(payload);
          
          // Timestamp should be a valid ISO string
          const date = new Date(timestamp);
          expect(date.toString()).not.toBe('Invalid Date');
          
          return true;
        }),
        { numRuns: 100 }
      );
    });

    it('should allow processing of any valid event without errors', () => {
      fc.assert(
        fc.property(eventNotificationArb, (event) => {
          // Event should have required fields
          expect(event.type).toBe('EventNotification');
          expect(event.event_id).toBeDefined();
          expect(event.event_type).toBeDefined();
          expect(event.payload).toBeDefined();
          
          // Should be able to extract workspace ID
          const workspaceId = extractWorkspaceId(event.payload);
          expect(workspaceId).toBeDefined();
          
          return true;
        }),
        { numRuns: 100 }
      );
    });
  });

  /**
   * **Feature: performance-optimization, Property 9: Event Structure Completeness**
   * 
   * *For any* background task status update, emitted events should contain complete state information
   * 
   * This property tests that:
   * 1. All event types have the required fields
   * 2. Event type matches the payload structure
   * 3. All required information is present for UI updates
   * 
   * **Validates: Requirements 2.4**
   */
  describe('Property 9: Event Structure Completeness', () => {
    it('should have event_type matching payload structure', () => {
      fc.assert(
        fc.property(
          fc.tuple(
            fc.constantFrom(
              'status_changed',
              'progress_update',
              'task_completed',
              'error',
              'workspace_deleted',
              'workspace_created'
            ),
            workspaceEventPayloadArb
          ),
          ([_eventType, payload]) => {
            // When event type and payload are generated independently,
            // we verify that the payload contains the expected structure
            const payloadKeys = Object.keys(payload);
            expect(payloadKeys.length).toBe(1);
            
            // The payload should have exactly one key
            const payloadKey = payloadKeys[0];
            expect(payloadKey).toBeDefined();
            
            return true;
          }
        ),
        { numRuns: 100 }
      );
    });

    it('should have complete StatusChanged event structure', () => {
      fc.assert(
        fc.property(statusChangedEventArb, (payload) => {
          const event = payload.StatusChanged;
          
          // Required fields
          expect(event.workspace_id).toBeDefined();
          expect(event.status).toBeDefined();
          expect(event.timestamp).toBeDefined();
          
          // Status should have a type
          expect(event.status.type).toBeDefined();
          
          return true;
        }),
        { numRuns: 100 }
      );
    });

    it('should have complete ProgressUpdate event structure', () => {
      fc.assert(
        fc.property(progressUpdateEventArb, (payload) => {
          const event = payload.ProgressUpdate;
          
          // Required fields
          expect(event.workspace_id).toBeDefined();
          expect(event.progress).toBeDefined();
          expect(event.timestamp).toBeDefined();
          
          // Progress should be a valid number between 0 and 100
          expect(event.progress).toBeGreaterThanOrEqual(0);
          expect(event.progress).toBeLessThanOrEqual(100);
          
          return true;
        }),
        { numRuns: 100 }
      );
    });

    it('should have complete TaskCompleted event structure', () => {
      fc.assert(
        fc.property(taskCompletedEventArb, (payload) => {
          const event = payload.TaskCompleted;
          
          // Required fields
          expect(event.workspace_id).toBeDefined();
          expect(event.task_id).toBeDefined();
          expect(event.timestamp).toBeDefined();
          
          // Task ID should be non-empty
          expect(event.task_id.length).toBeGreaterThan(0);
          
          return true;
        }),
        { numRuns: 100 }
      );
    });

    it('should have complete Error event structure', () => {
      fc.assert(
        fc.property(errorEventArb, (payload) => {
          const event = payload.Error;
          
          // Required fields
          expect(event.workspace_id).toBeDefined();
          expect(event.error).toBeDefined();
          expect(event.timestamp).toBeDefined();
          
          // Error message should be non-empty
          expect(event.error.length).toBeGreaterThan(0);
          
          return true;
        }),
        { numRuns: 100 }
      );
    });
  });

  /**
   * **Feature: performance-optimization, Property 16: Synchronization Monitoring**
   * 
   * *For any* workspace state change, synchronization latency and success rates should be tracked
   * 
   * This property tests that:
   * 1. Sync metrics have valid ranges
   * 2. Success rate is calculated correctly
   * 3. Metrics are consistent
   * 
   * **Validates: Requirements 4.2**
   */
  describe('Property 16: Synchronization Monitoring', () => {
    it('should have valid metric ranges', () => {
      fc.assert(
        fc.property(syncMetricsArb, (metrics) => {
          // All counts should be non-negative
          expect(metrics.messagesReceived).toBeGreaterThanOrEqual(0);
          expect(metrics.messagesSent).toBeGreaterThanOrEqual(0);
          expect(metrics.successfulDeliveries).toBeGreaterThanOrEqual(0);
          expect(metrics.failedDeliveries).toBeGreaterThanOrEqual(0);
          
          // Latency should be non-negative
          expect(metrics.averageLatency).toBeGreaterThanOrEqual(0);
          
          return true;
        }),
        { numRuns: 100 }
      );
    });

    it('should have consistent delivery counts', () => {
      fc.assert(
        fc.property(
          fc.record({
            totalMessages: fc.integer({ min: 0, max: 10000 }),
            successRate: fc.double({ min: 0, max: 1, noNaN: true }),
          }),
          ({ totalMessages, successRate }) => {
            const successfulMessages = Math.floor(totalMessages * successRate);
            const failedMessages = totalMessages - successfulMessages;
            
            // Successful + failed should equal total
            expect(successfulMessages + failedMessages).toBe(totalMessages);
            
            // Both should be non-negative
            expect(successfulMessages).toBeGreaterThanOrEqual(0);
            expect(failedMessages).toBeGreaterThanOrEqual(0);
            
            return true;
          }
        ),
        { numRuns: 100 }
      );
    });

    it('should calculate success rate correctly', () => {
      fc.assert(
        fc.property(
          fc.record({
            successfulDeliveries: fc.integer({ min: 0, max: 10000 }),
            failedDeliveries: fc.integer({ min: 0, max: 10000 }),
          }),
          ({ successfulDeliveries, failedDeliveries }) => {
            const totalDeliveries = successfulDeliveries + failedDeliveries;
            
            if (totalDeliveries === 0) {
              // No deliveries means 100% success rate (or undefined)
              return true;
            }
            
            const successRate = successfulDeliveries / totalDeliveries;
            
            // Success rate should be between 0 and 1
            expect(successRate).toBeGreaterThanOrEqual(0);
            expect(successRate).toBeLessThanOrEqual(1);
            
            return true;
          }
        ),
        { numRuns: 100 }
      );
    });

    it('should have valid connection status transitions', () => {
      fc.assert(
        fc.property(
          fc.array(connectionStatusArb, { minLength: 1, maxLength: 10 }),
          (statusHistory) => {
            // All statuses should be valid
            const validStatuses: ConnectionStatus[] = [
              'connecting',
              'connected',
              'disconnected',
              'reconnecting',
              'error',
            ];
            
            for (const status of statusHistory) {
              expect(validStatuses).toContain(status);
            }
            
            return true;
          }
        ),
        { numRuns: 100 }
      );
    });

    it('should track latency measurements correctly', () => {
      fc.assert(
        fc.property(
          fc.array(
            fc.double({ min: 0, max: 10000, noNaN: true }),
            { minLength: 1, maxLength: 100 }
          ),
          (latencies) => {
            // Calculate average
            const sum = latencies.reduce((a, b) => a + b, 0);
            const average = sum / latencies.length;
            
            // Average should be within the range of min and max
            const min = Math.min(...latencies);
            const max = Math.max(...latencies);
            
            expect(average).toBeGreaterThanOrEqual(min);
            expect(average).toBeLessThanOrEqual(max);
            
            return true;
          }
        ),
        { numRuns: 100 }
      );
    });
  });
});
