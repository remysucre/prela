WITH UserBadges AS (
    SELECT 
        U.Id AS UserId,
        U.DisplayName,
        COUNT(CASE WHEN B.Class = 1 THEN 1 END) AS GoldBadges,
        COUNT(CASE WHEN B.Class = 2 THEN 1 END) AS SilverBadges,
        COUNT(CASE WHEN B.Class = 3 THEN 1 END) AS BronzeBadges
    FROM 
        Users U
    LEFT JOIN 
        Badges B ON U.Id = B.UserId
    GROUP BY 
        U.Id, U.DisplayName
), 
PostStats AS (
    SELECT 
        P.OwnerUserId,
        COUNT(CASE WHEN P.PostTypeId = 1 THEN 1 END) AS Questions,
        COUNT(CASE WHEN P.PostTypeId = 2 THEN 1 END) AS Answers,
        SUM(P.Score) AS TotalScore,
        SUM(P.ViewCount) AS TotalViews,
        SUM(epoch_us(cast('2024-10-01 12:34:56' as timestamp)) - epoch_us(P.CreationDate))::DOUBLE / COUNT(P.CreationDate) / 1e6 AS AverageAgeInSeconds
    FROM 
        Posts P
    GROUP BY 
        P.OwnerUserId
), 
CombinedStats AS (
    SELECT 
        U.Id AS UserId,
        U.DisplayName,
        COALESCE(UB.GoldBadges, 0) AS GoldBadges,
        COALESCE(UB.SilverBadges, 0) AS SilverBadges,
        COALESCE(UB.BronzeBadges, 0) AS BronzeBadges,
        COALESCE(PS.Questions, 0) AS TotalQuestions,
        COALESCE(PS.Answers, 0) AS TotalAnswers,
        COALESCE(PS.TotalScore, 0) AS TotalScore,
        COALESCE(PS.TotalViews, 0) AS TotalViews,
        COALESCE(PS.AverageAgeInSeconds, 0) AS AveragePostAgeInSeconds
    FROM 
        Users U
    LEFT JOIN 
        UserBadges UB ON U.Id = UB.UserId
    LEFT JOIN 
        PostStats PS ON U.Id = PS.OwnerUserId
)
SELECT 
    *,
    CASE 
        WHEN TotalQuestions > 0 THEN TotalScore / TotalQuestions 
        ELSE 0 
    END AS AverageScorePerQuestion,
    CASE 
        WHEN AveragePostAgeInSeconds > 0 THEN TotalViews / AveragePostAgeInSeconds 
        ELSE 0 
    END AS ViewsPerSecond
FROM 
    CombinedStats
WHERE 
    GoldBadges > 0
ORDER BY 
    TotalScore DESC, 
    DisplayName ASC
LIMIT 10;