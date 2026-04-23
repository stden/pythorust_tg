# language: en

Feature: BFL Sales Bot — AI consultant for massage chairs
  As a sales manager
  I want to use an AI bot to consult customers
  So that I can increase massage chair sales

  Background:
    Given the MySQL database is configured
    And the OpenAI client is initialized

  # ========== User management ==========

  Scenario: Saving a new user
    Given a user with id 123456 and name "Ivan"
    When the bot saves the user to the database
    Then the user exists in table "bot_users"
    And the user name is "Ivan"

  Scenario: Updating an existing user
    Given a user with id 123456 already exists in the database
    And the user changed username to "ivan_new"
    When the bot saves the user to the database
    Then the user username is updated to "ivan_new"

  # ========== Message handling ==========

  Scenario: Saving an incoming message
    Given a user with id 123456 sent the message "Hello!"
    When the bot saves the message to the database
    Then the message is saved with direction "incoming"
    And the message text is "Hello!"

  Scenario: Saving an outgoing message
    Given the bot sends a reply to user 123456
    When the bot saves an outgoing message "Hi!"
    Then the message is saved with direction "outgoing"

  # ========== Session handling ==========

  Scenario: Creating a new session on /start
    Given a user with id 123456 sent /start
    When the bot creates a new session
    Then the session is created with state "greeting"
    And the session is active

  Scenario: Ending the previous session on a new /start
    Given user 123456 has an active session
    When the user sends /start again
    Then the previous session is deactivated
    And a new active session is created

  # ========== Conversation history ==========

  Scenario: Getting conversation history
    Given user 123456 sent 5 messages
    And the bot replied to each
    When conversation history is requested with limit 20
    Then 10 messages are returned
    And messages are sorted by time

  Scenario: History limit
    Given user 123456 has 50 messages in history
    When conversation history is requested with limit 10
    Then only the last 10 messages are returned

  # ========== Greeting ==========

  Scenario: /start sends a greeting
    Given a new user "Maria" sent /start
    When the bot handles the command
    Then the bot sends a greeting with name "Maria"
    And the bot asks clarifying questions
    And the bot recommends the Relaxio Premium lineup

  # ========== AI responses ==========

  Scenario: Generating an AI response to a pricing question
    Given the user asks "How much does a massage chair cost?"
    When the bot generates an AI response
    Then the response contains product lineup information
    And the response is a reasonable length

  Scenario: AI uses the system prompt
    When AI messages are built
    Then the first message has role "system"
    And the system prompt contains "Relaxio"

  Scenario: Conversation history is passed to AI
    Given the history contains 3 user messages
    And 3 bot replies
    When AI messages are built
    Then messages include the conversation history
    And roles alternate between "user" and "assistant"

  # ========== Error handling ==========

  Scenario: AI error returns a fallback message
    Given the OpenAI API is unavailable
    When the bot tries to generate a response
    Then the bot returns an error message
    And the message contains "Sorry"

  Scenario: Reconnecting to MySQL after connection loss
    Given the MySQL connection is lost
    When the bot tries to save a message
    Then the bot reconnects to the database
    And the message is saved successfully

  # ========== Product lineup ==========

  Scenario: R5 model information
    When the customer asks about the budget model
    Then the bot recommends Relaxio Premium R5
    And it indicates a price up to 120k

  Scenario: R7 model information
    When the customer asks about the mid-range segment
    Then the bot recommends Relaxio Premium R7
    And it indicates a price up to 200k
    And it mentions 4D massage and zero gravity

  Scenario: R9 model information
    When the customer asks about the top model
    Then the bot recommends Relaxio Premium R9
    And it indicates a price up to 300k
    And it mentions all premium features

  # ========== Sales stages ==========

  Scenario Outline: Sales funnel stages
    Given the customer is at stage "<stage>"
    When the bot replies to the customer
    Then the bot follows the strategy for stage "<stage>"

    Examples:
      | stage                 |
      | needs discovery       |
      | detail clarification  |
      | model presentation    |
      | objection handling    |
      | closing               |

  # ========== Purchase scenarios: typical customers ==========

  Scenario: Buying for yourself — office worker with back pain
    Given the customer "Alexey" works in an office 8+ hours a day
    And they have lower back and neck pain
    And the budget is up to 200 thousand rubles
    When the customer describes their problem
    Then the bot identifies a need for therapeutic massage
    And it recommends model R7 with a heating function
    And it highlights programs for back and neck

  Scenario: Buying as a gift for parents
    Given the customer "Maria" is looking for a gift for parents
    And the parents are 60+ years old
    And easy controls are important
    When the customer clarifies requirements
    Then the bot asks about the parents' health
    And it recommends a model with a simple remote
    And it mentions gift wrapping

  Scenario: Buying for a country house — premium customer
    Given the customer "Dmitry" is setting up a spa zone in a cottage
    And the budget is unlimited
    And a premium look is important
    When the customer asks for top options
    Then the bot presents the flagship model R9
    And it describes all premium features
    And it offers upholstery color options

  Scenario: Buying for a family — universal use
    Given the customer "Elena" is choosing a chair for the whole family
    And family height range is from 155 to 190 cm
    And the chair will be used by 4 people
    When the customer describes the situation
    Then the bot asks about height and weight range
    And it recommends a model with automatic height adjustment
    And it mentions user profile settings

  Scenario: Amateur athlete — recovery after workouts
    Given the customer "Igor" trains 4 times a week
    And muscle recovery is needed
    And deep massage is desired
    When the customer asks about a suitable model
    Then the bot recommends a model with intense 4D massage
    And it mentions a program for athletes
    And it describes the stretching function

  # ========== Objection handling scenarios ==========

  Scenario: Objection "Too expensive"
    Given the customer is interested in model R7
    But they say the price 200 thousand is too high
    When the bot handles the price objection
    Then the bot offers an installment plan with no overpayment
    And it compares the cost to massage therapist visits
    And it mentions a 3-year warranty

  Scenario: Objection "I need to think"
    Given the customer has received all info about the model
    But they say they need to think
    When the bot handles the objection
    Then the bot asks what concerns the customer
    And it offers a showroom test drive
    And it mentions a limited-time promotion

  Scenario: Objection "Competitors are cheaper"
    Given the customer compares with Chinese alternatives
    And they mention a price 2x lower
    When the bot responds to competitor comparison
    Then the bot explains the difference in mechanism quality
    And it mentions local warranty service in Russia
    And it suggests checking customer reviews

  Scenario: Objection "It takes too much space"
    Given the customer lives in a 45 sq.m apartment
    And they worry about the chair size
    When the bot responds to the size objection
    Then the bot provides the exact dimensions
    And it mentions a folding feature
    And it offers help with measurements

  Scenario: Objection "Not sure I'll use it"
    Given the customer doubts regular use
    And they worry the chair will gather dust
    When the bot works with this objection
    Then the bot provides usage statistics
    And it mentions in-app reminders
    And it offers a trial period

  # ========== Deal closing scenarios ==========

  Scenario: Successful closing with delivery to Moscow
    Given the customer chose model R7 in black
    And they are ready to buy
    And they are in Moscow
    When the bot closes the deal
    Then the bot asks for the delivery address
    And it offers free delivery and assembly
    And it asks for a convenient time

  Scenario: Closing with delivery to a region
    Given a customer from Novosibirsk chose a model
    And they are ready to place an order
    When the bot processes a regional order
    Then the bot calculates delivery cost
    And it provides delivery time estimates
    And it offers cargo insurance

  Scenario: Installment plan
    Given the customer wants model R9 for 280 thousand
    And they request an installment plan for 12 months
    When the bot processes an installment plan
    Then the bot calculates the monthly payment
    And it explains the no-overpayment terms
    And it requests application details

  Scenario: Corporate sale to an office
    Given the customer represents a company
    And they want to buy 3 chairs for a break room
    When the bot processes a B2B request
    Then the bot offers a corporate discount
    And it asks for billing details for the invoice
    And it offers an extended warranty

  # ========== Special cases ==========

  Scenario: Customer with special needs
    Given the customer buys a chair for someone with arthritis
    And a gentle massage is needed
    When the bot consults on special needs
    Then the bot recommends a model with adjustable intensity
    And it mentions air compression massage
    And it advises consulting a doctor

  Scenario: Returning customer after a long pause
    Given the customer talked 2 weeks ago
    And they returned with a decision to buy
    When the bot recognizes a returning customer
    Then the bot greets by name
    And it recalls the previously chosen model
    And it checks current promotions

  Scenario: Customer wants to compare all models
    Given the customer asks for a comparison table
    And the budget is not decided
    When the bot builds a comparison
    Then the bot sends a brief summary of models
    And it highlights key differences
    And it offers help choosing

  Scenario: Negative review of a previous experience
    Given the customer complains about a bad experience with another brand
    And they are skeptical
    When the bot handles a skeptic
    Then the bot expresses understanding
    And it explains Relaxio differences
    And it offers a showroom demo

  # ========== Night and weekend scenarios ==========

  Scenario: Message outside working hours
    Given the customer writes at 3am
    When the bot replies to a late-night message
    Then the bot consults as usual
    And it warns about callback time
    And it offers to leave contact for a manager

  Scenario: Urgent order before a holiday
    Given there are 5 days left until New Year
    And the customer wants to receive the chair as a gift in time
    When the bot handles an urgent order
    Then the bot checks stock availability
    And it offers express delivery
    And it states the order deadline
