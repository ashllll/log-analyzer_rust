import React, { useState } from 'react';
import { Button } from './ui/Button';
import { FormField, FormGroup } from './ui/FormField';
import { Input } from './ui/Input';
import { useErrorManagement } from '../hooks/useErrorManagement';

// Tauri API types
declare global {
  interface Window {
    __TAURI__?: {
      invoke: (command: string, args?: any) => Promise<any>;
    };
  }
}

interface UserFeedbackProps {
  isOpen: boolean;
  onClose: () => void;
  errorId?: string;
  errorMessage?: string;
  context?: Record<string, any>;
}

interface FeedbackData {
  rating: number;
  description: string;
  email: string;
  category: string;
  reproductionSteps: string;
}

/**
 * User feedback collection component for error scenarios
 */
export const UserFeedback: React.FC<UserFeedbackProps> = ({
  isOpen,
  onClose,
  errorId,
  errorMessage,
  context
}) => {
  const [feedback, setFeedback] = useState<FeedbackData>({
    rating: 0,
    description: '',
    email: '',
    category: 'bug',
    reproductionSteps: ''
  });
  const [isSubmitting, setIsSubmitting] = useState(false);
  const { setValidationErrors, clearValidationErrors, getFieldError, reportError } = useErrorManagement();

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    
    // Clear previous validation errors
    clearValidationErrors();
    
    // Validate form
    const errors: Record<string, string> = {};
    
    if (!feedback.description.trim()) {
      errors.description = 'Please describe what happened';
    }
    
    if (feedback.email && !/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(feedback.email)) {
      errors.email = 'Please enter a valid email address';
    }
    
    if (Object.keys(errors).length > 0) {
      setValidationErrors(errors);
      return;
    }

    setIsSubmitting(true);
    
    try {
      // Submit feedback to backend
      if (window.__TAURI__) {
        await window.__TAURI__.invoke('submit_user_feedback', {
          feedback: {
            ...feedback,
            errorId,
            errorMessage,
            context,
            timestamp: new Date().toISOString(),
            userAgent: navigator.userAgent,
            url: window.location.href
          }
        });
      }
      
      // Show success message
      await reportError('Thank you for your feedback! We will review it and work on improvements.', {
        severity: 'low',
        showToUser: true,
        recoverable: true
      });
      
      onClose();
    } catch (error) {
      await reportError(error as Error, {
        component: 'UserFeedback',
        userAction: 'submit_feedback',
        severity: 'medium',
        showToUser: true
      });
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleRatingClick = (rating: number) => {
    setFeedback(prev => ({ ...prev, rating }));
  };

  if (!isOpen) {
    return null;
  }

  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
      <div className="bg-white dark:bg-gray-800 rounded-lg shadow-xl max-w-md w-full mx-4 max-h-[90vh] overflow-y-auto">
        <div className="p-6">
          <div className="flex justify-between items-center mb-4">
            <h2 className="text-lg font-semibold text-gray-900 dark:text-gray-100">
              Help Us Improve
            </h2>
            <button
              onClick={onClose}
              className="text-gray-400 hover:text-gray-600 dark:hover:text-gray-300"
              aria-label="Close feedback form"
            >
              <svg className="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
              </svg>
            </button>
          </div>

          {errorMessage && (
            <div className="mb-4 p-3 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-md">
              <p className="text-sm text-red-700 dark:text-red-300">
                <strong>Error:</strong> {errorMessage}
              </p>
            </div>
          )}

          <form onSubmit={handleSubmit} className="space-y-4">
            <FormGroup title="Your Experience">
              {/* Rating */}
              <FormField label="How would you rate your experience?" required>
                <div className="flex space-x-2">
                  {[1, 2, 3, 4, 5].map((star) => (
                    <button
                      key={star}
                      type="button"
                      onClick={() => handleRatingClick(star)}
                      className={`p-1 rounded ${
                        star <= feedback.rating
                          ? 'text-yellow-400'
                          : 'text-gray-300 dark:text-gray-600'
                      } hover:text-yellow-400 transition-colors`}
                      aria-label={`Rate ${star} star${star !== 1 ? 's' : ''}`}
                    >
                      <svg className="h-6 w-6" fill="currentColor" viewBox="0 0 20 20">
                        <path d="M9.049 2.927c.3-.921 1.603-.921 1.902 0l1.07 3.292a1 1 0 00.95.69h3.462c.969 0 1.371 1.24.588 1.81l-2.8 2.034a1 1 0 00-.364 1.118l1.07 3.292c.3.921-.755 1.688-1.54 1.118l-2.8-2.034a1 1 0 00-1.175 0l-2.8 2.034c-.784.57-1.838-.197-1.539-1.118l1.07-3.292a1 1 0 00-.364-1.118L2.98 8.72c-.783-.57-.38-1.81.588-1.81h3.461a1 1 0 00.951-.69l1.07-3.292z" />
                      </svg>
                    </button>
                  ))}
                </div>
              </FormField>

              {/* Category */}
              <FormField label="Category" error={getFieldError('category')}>
                <select
                  value={feedback.category}
                  onChange={(e) => setFeedback(prev => ({ ...prev, category: e.target.value }))}
                  className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100"
                >
                  <option value="bug">Bug Report</option>
                  <option value="feature">Feature Request</option>
                  <option value="usability">Usability Issue</option>
                  <option value="performance">Performance Issue</option>
                  <option value="other">Other</option>
                </select>
              </FormField>

              {/* Description */}
              <FormField 
                label="What happened?" 
                required 
                error={getFieldError('description')}
                description="Please describe what you were trying to do and what went wrong"
              >
                <textarea
                  value={feedback.description}
                  onChange={(e) => setFeedback(prev => ({ ...prev, description: e.target.value }))}
                  rows={4}
                  className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 resize-vertical"
                  placeholder="Describe what happened..."
                />
              </FormField>

              {/* Reproduction Steps */}
              <FormField 
                label="Steps to reproduce (optional)"
                description="Help us understand how to reproduce this issue"
              >
                <textarea
                  value={feedback.reproductionSteps}
                  onChange={(e) => setFeedback(prev => ({ ...prev, reproductionSteps: e.target.value }))}
                  rows={3}
                  className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 resize-vertical"
                  placeholder="1. Click on...\n2. Enter...\n3. See error..."
                />
              </FormField>

              {/* Email */}
              <FormField 
                label="Email (optional)" 
                error={getFieldError('email')}
                description="We'll only use this to follow up on your feedback"
              >
                <Input
                  type="email"
                  value={feedback.email}
                  onChange={(e) => setFeedback(prev => ({ ...prev, email: e.target.value }))}
                  placeholder="your@email.com"
                />
              </FormField>
            </FormGroup>

            {/* Submit Buttons */}
            <div className="flex justify-end space-x-3 pt-4 border-t border-gray-200 dark:border-gray-700">
              <Button
                type="button"
                variant="secondary"
                onClick={onClose}
                disabled={isSubmitting}
              >
                Cancel
              </Button>
              <Button
                type="submit"
                disabled={isSubmitting}
                className="min-w-[100px]"
              >
                {isSubmitting ? 'Sending...' : 'Send Feedback'}
              </Button>
            </div>
          </form>
        </div>
      </div>
    </div>
  );
};

export default UserFeedback;