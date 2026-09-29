
WITH RankedPosts AS (
    SELECT 
        p.Id AS PostId,
        p.Title,
        p.CreationDate,
        p.Score,
        p.OwnerUserId,
        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC) AS UserPostRank
    FROM 
        Posts p
    WHERE 
        p.PostTypeId = 1 AND
        p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
),
UserAggregates AS (
    SELECT 
        u.Id AS UserId,
        u.DisplayName,
        COUNT(DISTINCT r.PostId) AS QuestionCount,
        SUM(r.Score) AS TotalScore
    FROM 
        Users u
    LEFT JOIN 
        RankedPosts r ON u.Id = r.OwnerUserId
    GROUP BY 
        u.Id, u.DisplayName
),
TopUsers AS (
    SELECT 
        UserId,
        DisplayName,
        QuestionCount,
        TotalScore,
        RANK() OVER (ORDER BY TotalScore DESC) AS ScoreRank
    FROM 
        UserAggregates
)
SELECT 
    tu.DisplayName,
    tu.QuestionCount,
    tu.TotalScore,
    CASE 
        WHEN tu.ScoreRank <= 10 THEN 'Top Contributor'
        ELSE 'Regular Contributor'
    END AS ContributorStatus,
    COALESCE((SELECT STRING_AGG(DISTINCT t.TagName, ', ' ORDER BY t.TagName)
               FROM Posts p
               JOIN UNNEST(string_to_array(p.Tags, ',')) AS t(TagName) ON t.TagName IS NOT NULL
               WHERE p.OwnerUserId = tu.UserId AND p.PostTypeId = 1), 'No Tags') AS TagsUsed
FROM 
    TopUsers tu
WHERE 
    tu.QuestionCount > 5
ORDER BY 
    tu.TotalScore DESC, tu.QuestionCount DESC;
