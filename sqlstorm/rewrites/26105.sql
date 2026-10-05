WITH RankedPosts AS (
    SELECT 
        p.Id AS PostId,
        p.Title,
        p.Body,
        p.CreationDate,
        p.ViewCount,
        p.Score,
        STRING_AGG(t.TagName, ', ') AS Tags,
        ROW_NUMBER() OVER (PARTITION BY CASE 
                                             WHEN pt.Name = 'Question' THEN 'Questions'
                                             WHEN pt.Name = 'Answer' THEN 'Answers'
                                             ELSE 'Other' 
                                          END 
                           ORDER BY p.CreationDate DESC, p.Id) AS RN
    FROM 
        Posts p
        JOIN PostTypes pt ON p.PostTypeId = pt.Id
        LEFT JOIN Tags t ON t.ExcerptPostId = p.Id
    WHERE 
        p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 YEAR'
    GROUP BY 
        p.Id, p.Title, p.Body, p.CreationDate, p.ViewCount, p.Score, pt.Name
),

UserActivity AS (
    SELECT 
        u.Id AS UserId,
        u.DisplayName,
        COUNT(DISTINCT p.Id) AS TotalPosts,
        SUM(CASE WHEN pt.Name = 'Answer' THEN 1 ELSE 0 END) AS TotalAnswers,
        SUM(CASE WHEN pt.Name = 'Question' THEN 1 ELSE 0 END) AS TotalQuestions
    FROM 
        Users u
        LEFT JOIN Posts p ON u.Id = p.OwnerUserId
        LEFT JOIN PostTypes pt ON p.PostTypeId = pt.Id
    WHERE 
        u.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '3 MONTH'
    GROUP BY 
        u.Id, u.DisplayName
)

SELECT 
    u.DisplayName,
    u.TotalPosts,
    u.TotalAnswers,
    u.TotalQuestions,
    rp.PostId,
    rp.Title,
    rp.Tags,
    rp.ViewCount,
    rp.Score
FROM 
    UserActivity u
JOIN 
    RankedPosts rp ON u.TotalPosts > 0
WHERE 
    rp.RN <= 5 
ORDER BY 
    u.TotalPosts DESC, 
    rp.ViewCount DESC;