WITH UserBadgeStats AS (
    SELECT 
        u.Id AS UserId,
        u.DisplayName,
        COUNT(b.Id) AS TotalBadges,
        SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
    FROM 
        Users u
    LEFT JOIN 
        Badges b ON u.Id = b.UserId
    GROUP BY 
        u.Id, u.DisplayName
),

QuestionStatistics AS (
    SELECT 
        p.OwnerUserId,
        COUNT(p.Id) AS TotalQuestions,
        COUNT(CASE WHEN p.AcceptedAnswerId IS NOT NULL THEN 1 END) AS AcceptedAnswers,
        SUM(epoch_us(p.LastActivityDate) - epoch_us(p.CreationDate))::DOUBLE / COUNT(p.Id) / 1e6 / 3600 AS AvgTimeToAnswer
    FROM 
        Posts p
    WHERE 
        p.PostTypeId = 1  
    GROUP BY 
        p.OwnerUserId
),

EnhancedUserStats AS (
    SELECT 
        u.UserId,
        u.DisplayName,
        u.TotalBadges,
        q.TotalQuestions,
        q.AcceptedAnswers,
        q.AvgTimeToAnswer
    FROM 
        UserBadgeStats u
    LEFT JOIN 
        QuestionStatistics q ON u.UserId = q.OwnerUserId
)

SELECT 
    eus.DisplayName,
    eus.TotalBadges,
    eus.TotalQuestions,
    eus.AcceptedAnswers,
    eus.AvgTimeToAnswer,
    COALESCE(eus.TotalQuestions * 1.0 / NULLIF(eus.AcceptedAnswers, 0), 0) AS QuestionsToAcceptedRatio,
    COALESCE(eus.AvgTimeToAnswer, 0) AS AverageTimeToFirstAnswerInHours
FROM 
    EnhancedUserStats eus
ORDER BY 
    eus.TotalBadges DESC, 
    eus.TotalQuestions DESC;